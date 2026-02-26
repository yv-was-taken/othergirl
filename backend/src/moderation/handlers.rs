use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    users::reputation::calculate_reputation,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub reported_id: Uuid,
    pub chat_id: Uuid,
    pub reason: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlockRequest {
    pub blocked_id: Uuid,
}

pub async fn create_report(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateReportRequest>,
) -> AppResult<Json<serde_json::Value>> {
    validate_report_reason(&payload.reason)?;

    if payload.reported_id == auth_user.user_id {
        return Err(AppError::BadRequest("cannot report yourself".to_owned()));
    }

    let chat = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT user_a_id, user_b_id
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(payload.chat_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("chat not found".to_owned()))?;

    let reporter_is_member = chat.0 == auth_user.user_id || chat.1 == auth_user.user_id;
    let reported_is_member = chat.0 == payload.reported_id || chat.1 == payload.reported_id;

    if !reporter_is_member || !reported_is_member {
        return Err(AppError::Forbidden(
            "report requires a real chat interaction".to_owned(),
        ));
    }

    let inserted = sqlx::query_as::<_, (Uuid,)>(
        r#"
        INSERT INTO reports (reporter_id, reported_id, chat_id, reason, details)
        VALUES ($1, $2, $3, $4, COALESCE($5, ''))
        ON CONFLICT (reporter_id, reported_id, chat_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.reported_id)
    .bind(payload.chat_id)
    .bind(payload.reason.as_str())
    .bind(payload.details)
    .fetch_optional(&state.db)
    .await?;

    if inserted.is_none() {
        return Err(AppError::Conflict(
            "you already reported this user for this chat".to_owned(),
        ));
    }

    let report_id = inserted.unwrap().0;

    recalculate_reputation(&state, payload.reported_id).await?;

    Ok(Json(serde_json::json!({
        "report_id": report_id,
        "reported": true
    })))
}

pub async fn create_block(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateBlockRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if payload.blocked_id == auth_user.user_id {
        return Err(AppError::BadRequest("cannot block yourself".to_owned()));
    }

    sqlx::query(
        r#"
        INSERT INTO blocks (blocker_id, blocked_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.blocked_id)
    .execute(&state.db)
    .await?;

    Ok(Json(
        serde_json::json!({"blocked": true, "blocked_id": payload.blocked_id}),
    ))
}

pub async fn delete_block(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        r#"
        DELETE FROM blocks
        WHERE blocker_id = $1 AND blocked_id = $2
        "#,
    )
    .bind(auth_user.user_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "unblocked": result.rows_affected() > 0,
        "user_id": user_id
    })))
}

pub async fn list_blocks(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let blocked = sqlx::query_as::<_, (Uuid, String, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT b.blocked_id, u.username, b.created_at
        FROM blocks b
        JOIN users u ON u.id = b.blocked_id
        WHERE b.blocker_id = $1
        ORDER BY b.created_at DESC
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "blocked_users": blocked.into_iter().map(|(id, username, created_at)| {
            serde_json::json!({"id": id, "username": username, "created_at": created_at})
        }).collect::<Vec<_>>()
    })))
}

async fn recalculate_reputation(state: &AppState, user_id: Uuid) -> AppResult<(f64, bool)> {
    let total_chats = sqlx::query_scalar::<_, f64>(
        r#"
        SELECT COUNT(*)::DOUBLE PRECISION
        FROM chats
        WHERE (user_a_id = $1 OR user_b_id = $1)
          AND started_at >= NOW() - INTERVAL '30 days'
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?
    .max(1.0);

    let unique_reporters_30d = sqlx::query_scalar::<_, f64>(
        r#"
        SELECT COUNT(DISTINCT reporter_id)::DOUBLE PRECISION
        FROM reports
        WHERE reported_id = $1
          AND created_at >= NOW() - INTERVAL '30 days'
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let score = calculate_reputation(unique_reporters_30d, total_chats, 1.0);

    let unique_reporters_24h = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT reporter_id)::BIGINT
        FROM reports
        WHERE reported_id = $1
          AND created_at >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let suspend = unique_reporters_24h >= 10;

    sqlx::query(
        r#"
        UPDATE users
        SET reputation_score = $2,
            is_suspended = CASE WHEN $3 THEN TRUE ELSE is_suspended END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(score)
    .bind(suspend)
    .execute(&state.db)
    .await?;

    let is_suspended = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT is_suspended
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    Ok((score, is_suspended))
}

fn validate_report_reason(reason: &str) -> AppResult<()> {
    const ALLOWED: &[&str] = &["spam", "harassment", "underage", "illegal", "other"];

    if ALLOWED.contains(&reason) {
        return Ok(());
    }

    Err(AppError::BadRequest("invalid report reason".to_owned()))
}
