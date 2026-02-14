use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    payments::{
        ledger::{self, TxType},
        stripe,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BuySparksRequest {
    #[validate(range(min = 1))]
    pub amount: i64,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct TxPagination {
    #[validate(range(min = 1))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CashoutConnectRequest {
    pub refresh_url: Option<String>,
    pub return_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CashoutRequest {
    #[validate(range(min = 1000))]
    pub amount: i64,
}

pub async fn subscribe(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<SubscribeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let stripe_secret = stripe_secret(&state)?;
    let price_id = state
        .config
        .stripe_premium_price_id
        .clone()
        .ok_or_else(|| AppError::Conflict("premium stripe price is not configured".to_owned()))?;

    let success_url = payload
        .success_url
        .unwrap_or_else(|| state.config.stripe_success_url.clone());
    let cancel_url = payload
        .cancel_url
        .unwrap_or_else(|| state.config.stripe_cancel_url.clone());

    let mut params = vec![
        ("mode".to_owned(), "subscription".to_owned()),
        ("line_items[0][price]".to_owned(), price_id),
        ("line_items[0][quantity]".to_owned(), "1".to_owned()),
        ("success_url".to_owned(), success_url.clone()),
        ("cancel_url".to_owned(), cancel_url.clone()),
        (
            "client_reference_id".to_owned(),
            auth_user.user_id.to_string(),
        ),
        (
            "metadata[user_id]".to_owned(),
            auth_user.user_id.to_string(),
        ),
        (
            "metadata[purchase_type]".to_owned(),
            "subscription".to_owned(),
        ),
    ];

    if let Some(email) = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT email
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .flatten()
    {
        params.push(("customer_email".to_owned(), email));
    }

    let session = stripe::create_checkout_session(stripe_secret.as_str(), &params).await?;
    let checkout_url = session
        .url
        .ok_or_else(|| AppError::Internal)?;

    Ok(Json(serde_json::json!({
        "provider": "stripe",
        "checkout_url": checkout_url,
        "session_id": session.id,
        "mode": "subscription"
    })))
}

pub async fn buy_sparks(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<BuySparksRequest>,
) -> AppResult<Json<serde_json::Value>> {
    payload.validate()?;

    let stripe_secret = stripe_secret(&state)?;
    let success_url = payload
        .success_url
        .unwrap_or_else(|| state.config.stripe_success_url.clone());
    let cancel_url = payload
        .cancel_url
        .unwrap_or_else(|| state.config.stripe_cancel_url.clone());

    let mut params = vec![
        ("mode".to_owned(), "payment".to_owned()),
        (
            "line_items[0][price_data][currency]".to_owned(),
            "usd".to_owned(),
        ),
        (
            "line_items[0][price_data][product_data][name]".to_owned(),
            format!("Sparks bundle ({})", payload.amount),
        ),
        (
            "line_items[0][price_data][unit_amount]".to_owned(),
            payload.amount.to_string(),
        ),
        ("line_items[0][quantity]".to_owned(), "1".to_owned()),
        ("success_url".to_owned(), success_url.clone()),
        ("cancel_url".to_owned(), cancel_url.clone()),
        (
            "client_reference_id".to_owned(),
            auth_user.user_id.to_string(),
        ),
        (
            "metadata[user_id]".to_owned(),
            auth_user.user_id.to_string(),
        ),
        (
            "metadata[purchase_type]".to_owned(),
            "sparks".to_owned(),
        ),
        (
            "metadata[sparks_amount]".to_owned(),
            payload.amount.to_string(),
        ),
    ];

    if let Some(email) = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT email
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .flatten()
    {
        params.push(("customer_email".to_owned(), email));
    }

    let session = stripe::create_checkout_session(stripe_secret.as_str(), &params).await?;
    let checkout_url = session
        .url
        .ok_or_else(|| AppError::Internal)?;

    Ok(Json(serde_json::json!({
        "provider": "stripe",
        "checkout_url": checkout_url,
        "session_id": session.id,
        "mode": "payment",
        "amount": payload.amount
    })))
}

pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let webhook_secret = state
        .config
        .stripe_webhook_secret
        .clone()
        .ok_or_else(|| AppError::Forbidden("stripe webhook secret is not configured".to_owned()))?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing stripe-signature header".to_owned()))?;

    stripe::verify_webhook_signature(body.as_ref(), signature, webhook_secret.as_str())?;

    let event: stripe::StripeEvent =
        serde_json::from_slice(body.as_ref()).map_err(|_| AppError::BadRequest(
            "invalid stripe webhook payload".to_owned(),
        ))?;

    let inserted = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO payment_webhook_events (stripe_event_id, event_type, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (stripe_event_id) DO NOTHING
        RETURNING stripe_event_id
        "#,
    )
    .bind(event.id.as_str())
    .bind(event.event_type.as_str())
    .bind(serde_json::from_slice::<serde_json::Value>(body.as_ref()).unwrap_or_default())
    .fetch_optional(&state.db)
    .await?;

    if inserted.is_none() {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "already_processed": true
        })));
    }

    match event.event_type.as_str() {
        "checkout.session.completed" => {
            let session: stripe::CheckoutSession = serde_json::from_value(event.data.object)
                .map_err(|_| AppError::BadRequest("invalid checkout session payload".to_owned()))?;

            if session.mode.as_deref() == Some("subscription") {
                apply_subscription_checkout(&state, session).await?;
            } else if session.mode.as_deref() == Some("payment") {
                apply_sparks_checkout(&state, session).await?;
            }
        }
        "customer.subscription.updated" | "customer.subscription.created" => {
            let subscription: stripe::StripeSubscription =
                serde_json::from_value(event.data.object).map_err(|_| {
                    AppError::BadRequest("invalid subscription payload".to_owned())
                })?;
            upsert_subscription_from_event(&state, &subscription).await?;
        }
        "customer.subscription.deleted" => {
            let subscription: stripe::StripeSubscription =
                serde_json::from_value(event.data.object).map_err(|_| {
                    AppError::BadRequest("invalid subscription payload".to_owned())
                })?;
            mark_subscription_canceled(&state, &subscription).await?;
        }
        _ => {}
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "event_id": event.id,
        "event_type": event.event_type
    })))
}

pub async fn balance(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let balance = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT sparks_balance
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    Ok(Json(serde_json::json!({"balance": balance})))
}

pub async fn transactions(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<TxPagination>,
) -> AppResult<Json<serde_json::Value>> {
    pagination.validate()?;

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let transactions =
        sqlx::query_as::<_, (Uuid, i64, String, Option<Uuid>, chrono::DateTime<Utc>)>(
            r#"
        SELECT id, amount, transaction_type, reference_id, created_at
        FROM spark_transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        )
        .bind(auth_user.user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let entries = transactions
        .into_iter()
        .map(|(id, amount, transaction_type, reference_id, created_at)| {
            serde_json::json!({
                "id": id,
                "amount": amount,
                "transaction_type": transaction_type,
                "reference_id": reference_id,
                "created_at": created_at
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(
        serde_json::json!({"transactions": entries, "page": page, "per_page": per_page}),
    ))
}

pub async fn cashout_connect(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CashoutConnectRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let stripe_secret = stripe_secret(&state)?;

    let existing_account = sqlx::query_scalar::<_, String>(
        r#"
        SELECT stripe_account_id
        FROM stripe_connect_accounts
        WHERE user_id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?;

    let account = if let Some(account_id) = existing_account {
        stripe::get_account(stripe_secret.as_str(), account_id.as_str()).await?
    } else {
        stripe::create_connect_account(stripe_secret.as_str()).await?
    };

    sqlx::query(
        r#"
        INSERT INTO stripe_connect_accounts (user_id, stripe_account_id, payouts_enabled)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id)
        DO UPDATE
        SET stripe_account_id = EXCLUDED.stripe_account_id,
            payouts_enabled = EXCLUDED.payouts_enabled
        "#,
    )
    .bind(auth_user.user_id)
    .bind(account.id.as_str())
    .bind(account.payouts_enabled)
    .execute(&state.db)
    .await?;

    let refresh_url = payload
        .refresh_url
        .unwrap_or_else(|| state.config.stripe_connect_refresh_url.clone());
    let return_url = payload
        .return_url
        .unwrap_or_else(|| state.config.stripe_connect_return_url.clone());

    let account_link = stripe::create_account_link(
        stripe_secret.as_str(),
        account.id.as_str(),
        refresh_url.as_str(),
        return_url.as_str(),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "provider": "stripe_connect",
        "stripe_account_id": account.id,
        "payouts_enabled": account.payouts_enabled,
        "onboarding_url": account_link.url
    })))
}

pub async fn cashout_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CashoutRequest>,
) -> AppResult<Json<serde_json::Value>> {
    payload.validate()?;

    let connect = sqlx::query_as::<_, (String, bool)>(
        r#"
        SELECT stripe_account_id, payouts_enabled
        FROM stripe_connect_accounts
        WHERE user_id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Conflict("connect account is not linked".to_owned()))?;

    if !connect.1 {
        return Err(AppError::Conflict(
            "connect account payouts are not enabled".to_owned(),
        ));
    }

    let mut tx = ledger::begin_tx(&state).await?;
    let balance = ledger::apply_spark_transaction(
        &mut tx,
        auth_user.user_id,
        -payload.amount,
        TxType::Cashout,
        None,
    )
    .await?;
    ledger::commit(tx).await?;

    let stripe_secret = stripe_secret(&state)?;
    let transfer_id = match stripe::create_transfer(
        stripe_secret.as_str(),
        connect.0.as_str(),
        payload.amount,
        "usd",
        "Othergirl cashout",
    )
    .await
    {
        Ok(transfer_id) => transfer_id,
        Err(err) => {
            let mut rollback_tx = ledger::begin_tx(&state).await?;
            let _ = ledger::apply_spark_transaction(
                &mut rollback_tx,
                auth_user.user_id,
                payload.amount,
                TxType::Cashout,
                None,
            )
            .await?;
            ledger::commit(rollback_tx).await?;
            return Err(err);
        }
    };

    Ok(Json(serde_json::json!({
        "requested": payload.amount,
        "new_balance": balance,
        "status": "processing",
        "stripe_transfer_id": transfer_id
    })))
}

pub async fn cashout_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let connect = sqlx::query_as::<_, (String, bool, chrono::DateTime<Utc>)>(
        r#"
        SELECT stripe_account_id, payouts_enabled, created_at
        FROM stripe_connect_accounts
        WHERE user_id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?;

    let connect = if let Some((account_id, _, created_at)) = connect {
        let stripe_secret = stripe_secret(&state)?;
        let account = stripe::get_account(stripe_secret.as_str(), account_id.as_str()).await?;

        sqlx::query(
            r#"
            UPDATE stripe_connect_accounts
            SET payouts_enabled = $2
            WHERE user_id = $1
            "#,
        )
        .bind(auth_user.user_id)
        .bind(account.payouts_enabled)
        .execute(&state.db)
        .await?;

        Some(serde_json::json!({
            "stripe_account_id": account.id,
            "payouts_enabled": account.payouts_enabled,
            "created_at": created_at
        }))
    } else {
        None
    };

    Ok(Json(serde_json::json!({ "connect": connect })))
}

async fn apply_subscription_checkout(
    state: &AppState,
    session: stripe::CheckoutSession,
) -> AppResult<()> {
    let subscription_id = session.subscription.clone().ok_or_else(|| {
        AppError::BadRequest("checkout session missing subscription id".to_owned())
    })?;
    let stripe_secret = stripe_secret(state)?;
    let subscription =
        stripe::get_subscription(stripe_secret.as_str(), subscription_id.as_str()).await?;

    let user_id = parse_user_id_from_session(&session)?;
    let customer_id = subscription
        .customer
        .or(session.customer)
        .unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            user_id, stripe_subscription_id, stripe_customer_id, status,
            current_period_start, current_period_end
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (stripe_subscription_id)
        DO UPDATE SET
            status = EXCLUDED.status,
            current_period_start = EXCLUDED.current_period_start,
            current_period_end = EXCLUDED.current_period_end
        "#,
    )
    .bind(user_id)
    .bind(subscription.id.as_str())
    .bind(customer_id)
    .bind(subscription.status.as_str())
    .bind(stripe::timestamp_to_utc(subscription.current_period_start))
    .bind(stripe::timestamp_to_utc(subscription.current_period_end))
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        UPDATE users
        SET is_premium = TRUE, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn apply_sparks_checkout(state: &AppState, session: stripe::CheckoutSession) -> AppResult<()> {
    if session.payment_status.as_deref() != Some("paid") {
        return Ok(());
    }

    let user_id = parse_user_id_from_session(&session)?;
    let sparks_amount = session
        .metadata
        .get("sparks_amount")
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("missing sparks_amount metadata".to_owned()))?;

    let inserted = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO stripe_completed_sessions (stripe_session_id, user_id, kind, amount)
        VALUES ($1, $2, 'sparks', $3)
        ON CONFLICT (stripe_session_id) DO NOTHING
        RETURNING stripe_session_id
        "#,
    )
    .bind(session.id.as_str())
    .bind(user_id)
    .bind(sparks_amount)
    .fetch_optional(&state.db)
    .await?;

    if inserted.is_none() {
        return Ok(());
    }

    let mut tx = ledger::begin_tx(state).await?;
    let _ = ledger::apply_spark_transaction(&mut tx, user_id, sparks_amount, TxType::Purchase, None)
        .await?;
    ledger::commit(tx).await?;

    Ok(())
}

async fn upsert_subscription_from_event(
    state: &AppState,
    subscription: &stripe::StripeSubscription,
) -> AppResult<()> {
    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT user_id
        FROM subscriptions
        WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(subscription.id.as_str())
    .fetch_optional(&state.db)
    .await?;

    let Some(user_id) = user_id else {
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = $2,
            current_period_start = $3,
            current_period_end = $4
        WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(subscription.id.as_str())
    .bind(subscription.status.as_str())
    .bind(stripe::timestamp_to_utc(subscription.current_period_start))
    .bind(stripe::timestamp_to_utc(subscription.current_period_end))
    .execute(&state.db)
    .await?;

    let premium_active = matches!(subscription.status.as_str(), "active" | "trialing" | "past_due");
    sqlx::query(
        r#"
        UPDATE users
        SET is_premium = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(premium_active)
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn mark_subscription_canceled(
    state: &AppState,
    subscription: &stripe::StripeSubscription,
) -> AppResult<()> {
    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT user_id
        FROM subscriptions
        WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(subscription.id.as_str())
    .fetch_optional(&state.db)
    .await?;

    let Some(user_id) = user_id else {
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'canceled'
        WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(subscription.id.as_str())
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        UPDATE users
        SET is_premium = FALSE, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(())
}

fn parse_user_id_from_session(session: &stripe::CheckoutSession) -> AppResult<Uuid> {
    let raw = session
        .metadata
        .get("user_id")
        .cloned()
        .or(session.client_reference_id.clone())
        .ok_or_else(|| AppError::BadRequest("missing user_id in session metadata".to_owned()))?;

    Uuid::parse_str(raw.as_str()).map_err(|_| AppError::BadRequest("invalid user_id metadata".to_owned()))
}

fn stripe_secret(state: &AppState) -> AppResult<String> {
    state
        .config
        .stripe_secret_key
        .clone()
        .ok_or_else(|| AppError::Conflict("stripe is not configured".to_owned()))
}
