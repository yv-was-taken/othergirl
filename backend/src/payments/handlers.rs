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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CashoutStatus {
    Pending,
    Completed,
    Failed,
}

impl CashoutStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for CashoutStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for CashoutStatus {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "pending" => Ok(Self::Pending),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

const CASHOUT_RECONCILE_INTERVAL_SECS: u64 = 60;
const CASHOUT_RECONCILE_MAX_BACKOFF_SECS: u64 = 15 * 60;
const CASHOUT_RECONCILE_STALE_SECS: i64 = 300;
const CASHOUT_RECONCILE_BATCH_SIZE: i64 = 50;
const CASHOUT_RECONCILE_MAX_RETRY_AGE_SECS: i64 = 20 * 60 * 60;
const CASHOUT_MANUAL_RECONCILE_REASON: &str =
    "manual reconciliation required: pending cashout exceeded idempotency retry window";

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
        .ok_or_else(|| AppError::ServiceUnavailable("premium stripe price is not configured".to_owned()))?;

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
    let checkout_url = session.url.ok_or_else(|| AppError::Internal)?;

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
        ("metadata[purchase_type]".to_owned(), "sparks".to_owned()),
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
    let checkout_url = session.url.ok_or_else(|| AppError::Internal)?;

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
    let webhook_secret =
        state.config.stripe_webhook_secret.clone().ok_or_else(|| {
            AppError::ServiceUnavailable("stripe webhook secret is not configured".to_owned())
        })?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing stripe-signature header".to_owned()))?;

    stripe::verify_webhook_signature(body.as_ref(), signature, webhook_secret.as_str())?;

    let event: stripe::StripeEvent = serde_json::from_slice(body.as_ref())
        .map_err(|_| AppError::BadRequest("invalid stripe webhook payload".to_owned()))?;

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
                serde_json::from_value(event.data.object)
                    .map_err(|_| AppError::BadRequest("invalid subscription payload".to_owned()))?;
            upsert_subscription_from_event(&state, &subscription).await?;
        }
        "customer.subscription.deleted" => {
            let subscription: stripe::StripeSubscription =
                serde_json::from_value(event.data.object)
                    .map_err(|_| AppError::BadRequest("invalid subscription payload".to_owned()))?;
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

    let cashout_request_id = Uuid::new_v4();
    let mut tx = ledger::begin_tx(&state).await?;
    let balance = ledger::apply_spark_transaction(
        &mut tx,
        auth_user.user_id,
        -payload.amount,
        TxType::Cashout,
        Some(cashout_request_id),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO cashout_requests (id, user_id, amount, stripe_account_id, status)
        VALUES ($1, $2, $3, $4, 'pending')
        "#,
    )
    .bind(cashout_request_id)
    .bind(auth_user.user_id)
    .bind(payload.amount)
    .bind(connect.0.as_str())
    .execute(&mut *tx)
    .await
    .map_err(map_cashout_insert_error)?;

    ledger::commit(tx).await?;

    let stripe_secret = stripe_secret(&state)?;
    let idempotency_key = cashout_request_id.to_string();
    let transfer_id = match stripe::create_transfer(
        stripe_secret.as_str(),
        connect.0.as_str(),
        payload.amount,
        "usd",
        "Othergirl cashout",
        idempotency_key.as_str(),
    )
    .await
    {
        Ok(transfer_id) => transfer_id,
        Err(err) => {
            if should_finalize_failed_cashout(&err) {
                let failure_reason = app_error_message(&err);
                finalize_failed_cashout(
                    &state,
                    auth_user.user_id,
                    cashout_request_id,
                    payload.amount,
                    failure_reason.as_str(),
                )
                .await?;
                return Err(err);
            }

            let failure_reason = app_error_message(&err);
            if let Err(update_err) =
                mark_cashout_retry_pending(&state, cashout_request_id, failure_reason.as_str())
                    .await
            {
                tracing::error!(
                    "failed to update retry timestamp for cashout request {cashout_request_id}: {update_err}"
                );
            }

            tracing::warn!(
                "cashout request {cashout_request_id} accepted as pending after retriable stripe error"
            );
            return Ok(Json(serde_json::json!({
                "cashout_request_id": cashout_request_id,
                "requested": payload.amount,
                "new_balance": balance,
                "status": CashoutStatus::Pending.as_str()
            })));
        }
    };

    let updated =
        finalize_completed_cashout(&state, cashout_request_id, transfer_id.as_str()).await?;
    if !updated {
        let outcome = fetch_cashout_request_outcome(&state, cashout_request_id).await?;
        match outcome {
            Some((status, stored_transfer_id)) if status == CashoutStatus::Completed => {
                return Ok(Json(serde_json::json!({
                    "cashout_request_id": cashout_request_id,
                    "requested": payload.amount,
                    "new_balance": balance,
                    "status": CashoutStatus::Completed.as_str(),
                    "stripe_transfer_id": stored_transfer_id.unwrap_or(transfer_id)
                })));
            }
            Some((status, _)) if status == CashoutStatus::Pending => {
                return Err(AppError::Conflict(
                    "cashout request is still pending".to_owned(),
                ));
            }
            Some((status, _)) if status == CashoutStatus::Failed => {
                return Err(AppError::Conflict("cashout request failed".to_owned()));
            }
            Some(_) => {
                return Err(AppError::Internal);
            }
            None => {
                return Err(AppError::Internal);
            }
        }
    }

    Ok(Json(serde_json::json!({
        "cashout_request_id": cashout_request_id,
        "requested": payload.amount,
        "new_balance": balance,
        "status": CashoutStatus::Completed.as_str(),
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

pub fn spawn_cashout_reconciler(state: AppState, cancel: tokio_util::sync::CancellationToken) {
    tokio::spawn(async move {
        let base_interval = std::time::Duration::from_secs(CASHOUT_RECONCILE_INTERVAL_SECS);
        let max_backoff = std::time::Duration::from_secs(CASHOUT_RECONCILE_MAX_BACKOFF_SECS);
        let mut consecutive_rate_limits: u32 = 0;

        loop {
            match reconcile_stale_pending_cashouts(&state).await {
                Ok(rate_limited) if rate_limited => {
                    consecutive_rate_limits = consecutive_rate_limits.saturating_add(1);
                    let backoff = base_interval
                        .saturating_mul(1 << consecutive_rate_limits.min(6))
                        .min(max_backoff);
                    tracing::warn!(
                        "cashout reconciler rate-limited by stripe, backing off for {}s",
                        backoff.as_secs()
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {}
                        _ = cancel.cancelled() => break,
                    }
                }
                Ok(_) => {
                    consecutive_rate_limits = 0;
                    tokio::select! {
                        _ = tokio::time::sleep(base_interval) => {}
                        _ = cancel.cancelled() => break,
                    }
                }
                Err(err) => {
                    tracing::error!("cashout reconciliation failed: {err}");
                    consecutive_rate_limits = 0;
                    tokio::select! {
                        _ = tokio::time::sleep(base_interval) => {}
                        _ = cancel.cancelled() => break,
                    }
                }
            }
        }

        tracing::info!("cashout reconciler stopped");
    });
}

const CASHOUT_RECONCILER_LOCK_ID: i64 = 0x4F47_CA50; // "OG_CASHOUT" in hex-ish

/// Returns `Ok(true)` if Stripe rate-limited us during this pass (caller should back off).
async fn reconcile_stale_pending_cashouts(state: &AppState) -> AppResult<bool> {
    let locked = sqlx::query_scalar::<_, bool>(
        "SELECT pg_try_advisory_lock($1)",
    )
    .bind(CASHOUT_RECONCILER_LOCK_ID)
    .fetch_one(&state.db)
    .await?;

    if !locked {
        return Ok(false);
    }

    let result = reconcile_stale_pending_cashouts_inner(state).await;

    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(CASHOUT_RECONCILER_LOCK_ID)
        .execute(&state.db)
        .await;

    result
}

async fn reconcile_stale_pending_cashouts_inner(state: &AppState) -> AppResult<bool> {
    let resolved = mark_old_pending_cashouts_for_manual_reconciliation(state).await?;
    if resolved > 0 {
        tracing::info!(
            "resolved {resolved} stale pending cashout request(s) during manual reconciliation pass"
        );
    }

    let pending = sqlx::query_as::<_, (Uuid, Uuid, i64, String, chrono::DateTime<Utc>)>(
        r#"
        SELECT id, user_id, amount, stripe_account_id, updated_at
        FROM cashout_requests
        WHERE status = 'pending'
          AND updated_at < NOW() - ($1::bigint * INTERVAL '1 second')
          AND created_at >= NOW() - ($3::bigint * INTERVAL '1 second')
        ORDER BY updated_at ASC, created_at ASC
        LIMIT $2
        "#,
    )
    .bind(CASHOUT_RECONCILE_STALE_SECS)
    .bind(CASHOUT_RECONCILE_BATCH_SIZE)
    .bind(CASHOUT_RECONCILE_MAX_RETRY_AGE_SECS)
    .fetch_all(&state.db)
    .await?;

    if pending.is_empty() {
        return Ok(false);
    }

    let stripe_secret = match stripe_secret(state) {
        Ok(secret) => secret,
        Err(err) => {
            tracing::warn!("cashout reconciliation skipped: {err}");
            return Ok(false);
        }
    };

    let mut was_rate_limited = false;
    for (cashout_request_id, user_id, amount, stripe_account_id, expected_updated_at) in pending {
        if was_rate_limited {
            break;
        }
        match reconcile_pending_cashout(
            state,
            stripe_secret.as_str(),
            cashout_request_id,
            user_id,
            amount,
            stripe_account_id.as_str(),
            expected_updated_at,
        )
        .await
        {
            Ok(()) => {}
            Err(AppError::TooManyRequests(_)) => {
                was_rate_limited = true;
            }
            Err(err) => {
                tracing::error!(
                    "cashout reconciliation failed for request {cashout_request_id}: {err}"
                );
            }
        }
    }

    Ok(was_rate_limited)
}

async fn reconcile_pending_cashout(
    state: &AppState,
    stripe_secret: &str,
    cashout_request_id: Uuid,
    user_id: Uuid,
    amount: i64,
    stripe_account_id: &str,
    expected_updated_at: chrono::DateTime<Utc>,
) -> AppResult<()> {
    let claimed =
        claim_pending_cashout_for_reconciliation(state, cashout_request_id, expected_updated_at)
            .await?;
    if !claimed {
        return Ok(());
    }

    let idempotency_key = cashout_request_id.to_string();
    match stripe::create_transfer(
        stripe_secret,
        stripe_account_id,
        amount,
        "usd",
        "Othergirl cashout",
        idempotency_key.as_str(),
    )
    .await
    {
        Ok(transfer_id) => {
            let updated =
                finalize_completed_cashout(state, cashout_request_id, transfer_id.as_str()).await?;
            if !updated {
                tracing::error!(
                    "cashout request {cashout_request_id} was claimed for reconciliation but could not be finalized completed"
                );
                return Err(AppError::Conflict(
                    "cashout reconciliation completion lost ownership".to_owned(),
                ));
            }
        }
        Err(err) => {
            if should_finalize_failed_cashout(&err) {
                let failure_reason = app_error_message(&err);
                finalize_failed_cashout(
                    state,
                    user_id,
                    cashout_request_id,
                    amount,
                    failure_reason.as_str(),
                )
                .await?;
            } else {
                let failure_reason = app_error_message(&err);
                mark_cashout_retry_pending(state, cashout_request_id, failure_reason.as_str())
                    .await?;
                tracing::warn!(
                    "cashout request {cashout_request_id} reconciliation deferred after retriable stripe error"
                );
            }
        }
    }

    Ok(())
}

async fn claim_pending_cashout_for_reconciliation(
    state: &AppState,
    cashout_request_id: Uuid,
    expected_updated_at: chrono::DateTime<Utc>,
) -> AppResult<bool> {
    let claim = sqlx::query(
        r#"
        UPDATE cashout_requests
        SET updated_at = NOW()
        WHERE id = $1
          AND status = $2
          AND updated_at = $3
        "#,
    )
    .bind(cashout_request_id)
    .bind(CashoutStatus::Pending.as_str())
    .bind(expected_updated_at)
    .execute(&state.db)
    .await?;

    Ok(claim.rows_affected() == 1)
}

async fn finalize_completed_cashout(
    state: &AppState,
    cashout_request_id: Uuid,
    transfer_id: &str,
) -> AppResult<bool> {
    let completion = sqlx::query(
        r#"
        UPDATE cashout_requests
        SET status = 'completed',
            stripe_transfer_id = $2,
            completed_at = NOW(),
            updated_at = NOW(),
            failure_reason = NULL
        WHERE id = $1 AND status = 'pending'
        "#,
    )
    .bind(cashout_request_id)
    .bind(transfer_id)
    .execute(&state.db)
    .await?;

    Ok(completion.rows_affected() == 1)
}

async fn finalize_failed_cashout(
    state: &AppState,
    user_id: Uuid,
    cashout_request_id: Uuid,
    amount: i64,
    failure_reason: &str,
) -> AppResult<()> {
    let mut tx = ledger::begin_tx(state).await?;

    // Keep lock ordering consistent with cashout_request:
    // always lock the user row before touching cashout_requests rows.
    let _ = ledger::current_balance(&mut tx, user_id).await?;

    let updated = sqlx::query(
        r#"
        UPDATE cashout_requests
        SET status = 'failed',
            failure_reason = $2,
            failed_at = NOW(),
            updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        "#,
    )
    .bind(cashout_request_id)
    .bind(failure_reason)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() == 0 {
        ledger::commit(tx).await?;
        return Ok(());
    }

    let _ = ledger::apply_spark_transaction(
        &mut tx,
        user_id,
        amount,
        TxType::CashoutRefund,
        Some(cashout_request_id),
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE cashout_requests
        SET refunded_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(cashout_request_id)
    .execute(&mut *tx)
    .await?;

    ledger::commit(tx).await
}

fn app_error_message(err: &AppError) -> String {
    match err {
        AppError::BadRequest(message)
        | AppError::Unauthorized(message)
        | AppError::Forbidden(message)
        | AppError::NotFound(message)
        | AppError::Conflict(message)
        | AppError::TooManyRequests(message)
        | AppError::ServiceUnavailable(message) => message.clone(),
        AppError::Internal => "internal server error".to_owned(),
    }
}

fn map_cashout_insert_error(err: sqlx::Error) -> AppError {
    if is_unique_violation(&err) {
        return AppError::Conflict("a cashout request is already pending".to_owned());
    }

    AppError::from(err)
}

async fn fetch_cashout_request_outcome(
    state: &AppState,
    cashout_request_id: Uuid,
) -> AppResult<Option<(CashoutStatus, Option<String>)>> {
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        r#"
        SELECT status, stripe_transfer_id
        FROM cashout_requests
        WHERE id = $1
        "#,
    )
    .bind(cashout_request_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(|(status, transfer_id)| {
        let parsed = CashoutStatus::try_from(status.as_str()).unwrap_or(CashoutStatus::Pending);
        (parsed, transfer_id)
    }))
}

async fn mark_cashout_retry_pending(
    state: &AppState,
    cashout_request_id: Uuid,
    failure_reason: &str,
) -> AppResult<()> {
    let _ = sqlx::query(
        r#"
        UPDATE cashout_requests
        SET updated_at = NOW(),
            failure_reason = $2
        WHERE id = $1
          AND status = $3
        "#,
    )
    .bind(cashout_request_id)
    .bind(failure_reason)
    .bind(CashoutStatus::Pending.as_str())
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn mark_old_pending_cashouts_for_manual_reconciliation(state: &AppState) -> AppResult<u64> {
    let stale = sqlx::query_as::<_, (Uuid, Uuid, i64, String)>(
        r#"
        SELECT id, user_id, amount, stripe_account_id
        FROM cashout_requests
        WHERE status = $1
          AND created_at < NOW() - ($2::bigint * INTERVAL '1 second')
        LIMIT $3
        "#,
    )
    .bind(CashoutStatus::Pending.as_str())
    .bind(CASHOUT_RECONCILE_MAX_RETRY_AGE_SECS)
    .bind(CASHOUT_RECONCILE_BATCH_SIZE)
    .fetch_all(&state.db)
    .await?;

    if stale.is_empty() {
        return Ok(0);
    }

    let stripe_secret = match stripe_secret(state) {
        Ok(secret) => secret,
        Err(err) => {
            tracing::error!(
                "cannot verify stale cashouts with stripe, skipping manual reconciliation: {err}"
            );
            return Ok(0);
        }
    };

    let mut count = 0u64;
    for (cashout_id, user_id, amount, stripe_account_id) in &stale {
        let idempotency_key = cashout_id.to_string();
        match stripe::create_transfer(
            stripe_secret.as_str(),
            stripe_account_id.as_str(),
            *amount,
            "usd",
            "Othergirl cashout",
            idempotency_key.as_str(),
        )
        .await
        {
            Ok(transfer_id) => {
                match finalize_completed_cashout(state, *cashout_id, transfer_id.as_str()).await {
                    Ok(true) => {
                        tracing::info!(
                            "stale cashout {cashout_id} verified as completed via stripe replay (transfer {transfer_id})"
                        );
                    }
                    Ok(false) => {
                        tracing::warn!(
                            "stale cashout {cashout_id} stripe replay succeeded but row was already finalized"
                        );
                    }
                    Err(err) => {
                        tracing::error!(
                            "stale cashout {cashout_id} stripe replay succeeded but db finalization failed: {err}"
                        );
                        continue;
                    }
                }
                count += 1;
            }
            Err(ref err) if should_finalize_failed_cashout(err) => {
                let failure_reason = app_error_message(err);
                if let Err(refund_err) = finalize_failed_cashout(
                    state,
                    *user_id,
                    *cashout_id,
                    *amount,
                    failure_reason.as_str(),
                )
                .await
                {
                    tracing::error!(
                        "failed to finalize stale pending cashout {cashout_id} after confirmed stripe failure: {refund_err}"
                    );
                    continue;
                }
                count += 1;
            }
            Err(err) => {
                let failure_reason = app_error_message(&err);
                tracing::error!(
                    "stale cashout {cashout_id} could not be verified with stripe ({failure_reason}), requires manual investigation"
                );
                if let Err(update_err) = mark_cashout_retry_pending(
                    state,
                    *cashout_id,
                    &format!("{CASHOUT_MANUAL_RECONCILE_REASON}: {failure_reason}"),
                )
                .await
                {
                    tracing::error!(
                        "failed to update failure reason for stale cashout {cashout_id}: {update_err}"
                    );
                }
            }
        }
    }

    Ok(count)
}

fn should_finalize_failed_cashout(err: &AppError) -> bool {
    matches!(
        err,
        AppError::BadRequest(_) | AppError::Unauthorized(_) | AppError::Forbidden(_)
    )
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|db_err| db_err.code())
        .is_some_and(|code| code == "23505")
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

async fn apply_sparks_checkout(
    state: &AppState,
    session: stripe::CheckoutSession,
) -> AppResult<()> {
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
    let _ =
        ledger::apply_spark_transaction(&mut tx, user_id, sparks_amount, TxType::Purchase, None)
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

    let premium_active = matches!(
        subscription.status.as_str(),
        "active" | "trialing" | "past_due"
    );
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

    Uuid::parse_str(raw.as_str())
        .map_err(|_| AppError::BadRequest("invalid user_id metadata".to_owned()))
}

fn stripe_secret(state: &AppState) -> AppResult<String> {
    state
        .config
        .stripe_secret_key
        .clone()
        .ok_or_else(|| AppError::ServiceUnavailable("stripe is not configured".to_owned()))
}
