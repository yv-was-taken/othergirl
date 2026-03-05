use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

fn require_admin(state: &AppState, auth_user: &AuthUser) -> AppResult<()> {
    if !state.config.admin_user_ids.contains(&auth_user.user_id) {
        return Err(AppError::Forbidden("admin access required".to_owned()));
    }
    Ok(())
}

pub async fn system_stats(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&state, &auth_user)?;

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM users")
        .fetch_one(&state.db)
        .await?;

    let active_chats =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM chats WHERE ended_at IS NULL")
            .fetch_one(&state.db)
            .await?;

    let total_messages = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM messages")
        .fetch_one(&state.db)
        .await?;

    let total_awards = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM awards")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "total_users": total_users,
        "active_chats": active_chats,
        "total_messages": total_messages,
        "total_awards": total_awards,
    })))
}

#[derive(Debug, Deserialize)]
pub struct UserListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_users(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<UserListParams>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&state, &auth_user)?;

    let per_page = params.per_page.unwrap_or(50).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let users = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Option<String>,
            bool,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, username, email, is_premium, is_suspended, created_at
        FROM users
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = users
        .into_iter()
        .map(
            |(id, username, email, is_premium, is_suspended, created_at)| {
                serde_json::json!({
                    "id": id,
                    "username": username,
                    "email": email,
                    "is_premium": is_premium,
                    "is_suspended": is_suspended,
                    "created_at": created_at,
                })
            },
        )
        .collect();

    Ok(Json(serde_json::json!({
        "users": items,
        "page": page,
        "per_page": per_page,
    })))
}

pub async fn suspend_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&state, &auth_user)?;

    if state.config.admin_user_ids.contains(&id) {
        return Err(AppError::BadRequest(
            "cannot suspend an admin user".to_owned(),
        ));
    }

    let result =
        sqlx::query("UPDATE users SET is_suspended = TRUE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found".to_owned()));
    }

    Ok(Json(serde_json::json!({
        "message": "user suspended",
        "user_id": id,
    })))
}

pub async fn unsuspend_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&state, &auth_user)?;

    let result =
        sqlx::query("UPDATE users SET is_suspended = FALSE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found".to_owned()));
    }

    Ok(Json(serde_json::json!({
        "message": "user unsuspended",
        "user_id": id,
    })))
}
