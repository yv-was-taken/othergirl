use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    notifications::models::{Notification, NotificationPagination},
    AppState,
};

pub async fn list_notifications(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<NotificationPagination>,
) -> AppResult<Json<serde_json::Value>> {
    pagination.validate()?;

    let limit = pagination.limit.unwrap_or(50).clamp(1, 100);
    let offset = pagination.offset.unwrap_or(0).max(0);

    let notifications = sqlx::query_as::<_, Notification>(
        r#"
        SELECT id, user_id, notification_type, title, body, is_read, metadata, created_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth_user.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "notifications": notifications })))
}

pub async fn mark_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = TRUE WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("notification not found".into()));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = TRUE WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(auth_user.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "updated": result.rows_affected() })))
}

pub async fn unread_count(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(auth_user.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "count": row.0 })))
}
