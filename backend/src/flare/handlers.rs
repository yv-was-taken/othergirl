use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    flare::models::FlareItem,
    payments::ledger::{self, TxType},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct StoreFilter {
    pub item_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseItemRequest {
    pub item_id: Uuid,
}

pub async fn list_items(
    State(state): State<AppState>,
    Query(filter): Query<StoreFilter>,
) -> AppResult<Json<serde_json::Value>> {
    let items = if let Some(item_type) = filter.item_type {
        sqlx::query_as::<_, FlareItem>(
            r#"
            SELECT id, name, description, item_type, price_sparks, rarity, asset_data, is_active, created_at
            FROM flare_items
            WHERE is_active = TRUE AND item_type = $1
            ORDER BY rarity DESC, price_sparks ASC
            "#,
        )
        .bind(item_type)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, FlareItem>(
            r#"
            SELECT id, name, description, item_type, price_sparks, rarity, asset_data, is_active, created_at
            FROM flare_items
            WHERE is_active = TRUE
            ORDER BY rarity DESC, price_sparks ASC
            "#,
        )
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(serde_json::json!({"items": items})))
}

pub async fn purchase_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<PurchaseItemRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = ledger::begin_tx(&state).await?;

    let row = sqlx::query_as::<_, (Uuid, i64, bool)>(
        r#"
        SELECT id, price_sparks, is_active
        FROM flare_items
        WHERE id = $1
        "#,
    )
    .bind(payload.item_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("flare item not found".to_owned()))?;

    if !row.2 {
        return Err(AppError::Conflict("flare item is inactive".to_owned()));
    }

    let already_owned = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM user_flare
            WHERE user_id = $1 AND flare_item_id = $2
        )
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.item_id)
    .fetch_one(&mut *tx)
    .await?;

    if already_owned {
        return Err(AppError::Conflict("flare item already owned".to_owned()));
    }

    let balance = ledger::apply_spark_transaction(
        &mut tx,
        auth_user.user_id,
        -row.1,
        TxType::FlarePurchase,
        Some(payload.item_id),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_flare (user_id, flare_item_id, is_equipped)
        VALUES ($1, $2, FALSE)
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.item_id)
    .execute(&mut *tx)
    .await?;

    ledger::commit(tx).await?;

    Ok(Json(serde_json::json!({
        "purchased": true,
        "item_id": payload.item_id,
        "new_balance": balance
    })))
}
