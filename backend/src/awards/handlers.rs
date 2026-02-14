use axum::{extract::State, Json};
use validator::Validate;

use crate::{
    auth::middleware::AuthUser,
    awards::{
        models::{Award, SendAwardRequest},
        service,
    },
    error::AppResult,
    AppState,
};

pub async fn send_award(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<SendAwardRequest>,
) -> AppResult<Json<serde_json::Value>> {
    payload.validate()?;

    let outcome = service::send_award_internal(
        &state,
        auth_user.user_id,
        payload.chat_id,
        payload.award_type,
        payload.spark_amount,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "award_id": outcome.award_id,
        "recipient_id": outcome.recipient_id,
        "award_type": outcome.award_type,
        "spark_amount": outcome.spark_amount,
        "recipient_cut": outcome.recipient_cut,
        "sender_balance": outcome.sender_balance
    })))
}

pub async fn list_awards(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let awards = sqlx::query_as::<_, Award>(
        r#"
        SELECT id, sender_id, recipient_id, chat_id, award_type, spark_amount, recipient_cut, created_at
        FROM awards
        WHERE recipient_id = $1 OR sender_id = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"awards": awards})))
}
