use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::middleware::AuthUser,
    chat::{
        encryption,
        models::{ChatDetails, ChatMessage, ChatSummary},
    },
    error::{AppError, AppResult},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct Pagination {
    #[validate(range(min = 1))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<i64>,
}

pub async fn list_chats(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<Vec<ChatSummary>>> {
    pagination.validate()?;

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let chats = sqlx::query_as::<_, ChatSummary>(
        r#"
        SELECT
            c.id,
            CASE WHEN c.user_a_id = $1 THEN c.user_b_id ELSE c.user_a_id END AS partner_id,
            u.username AS partner_username,
            c.started_at,
            c.ended_at
        FROM chats c
        JOIN users u
            ON u.id = CASE WHEN c.user_a_id = $1 THEN c.user_b_id ELSE c.user_a_id END
        WHERE c.user_a_id = $1 OR c.user_b_id = $1
        ORDER BY c.started_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth_user.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(chats))
}

pub async fn list_kept_chats(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Json<Vec<ChatSummary>>> {
    pagination.validate()?;

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let chats = sqlx::query_as::<_, ChatSummary>(
        r#"
        SELECT
            c.id,
            CASE WHEN c.user_a_id = $1 THEN c.user_b_id ELSE c.user_a_id END AS partner_id,
            u.username AS partner_username,
            c.started_at,
            c.ended_at
        FROM chats c
        JOIN users u
            ON u.id = CASE WHEN c.user_a_id = $1 THEN c.user_b_id ELSE c.user_a_id END
        WHERE (c.user_a_id = $1 OR c.user_b_id = $1)
          AND c.is_kept = TRUE
        ORDER BY c.started_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth_user.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(chats))
}

pub async fn get_chat(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(chat_id): Path<Uuid>,
) -> AppResult<Json<ChatDetails>> {
    let chat = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"
        SELECT user_a_id, user_b_id, started_at, ended_at
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("chat not found".to_owned()))?;

    if chat.0 != auth_user.user_id && chat.1 != auth_user.user_id {
        return Err(AppError::Forbidden(
            "not authorized to read this chat".to_owned(),
        ));
    }

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
            Option<String>,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, chat_id, sender_id, content_encrypted, nonce, content_text, is_read, created_at
        FROM messages
        WHERE chat_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(chat_id)
    .fetch_all(&state.db)
    .await?;

    let mut messages = Vec::with_capacity(rows.len());
    for (id, row_chat_id, sender_id, content_encrypted, nonce, content_text, is_read, created_at) in
        rows
    {
        let content = encryption::decrypt_for_chat(
            &state.db,
            chat_id,
            content_encrypted,
            nonce,
            content_text,
            &state.config.chat_key_encryption_key,
        )
        .await?;

        messages.push(ChatMessage {
            id,
            chat_id: row_chat_id,
            sender_id,
            content,
            is_read,
            created_at,
        });
    }

    Ok(Json(ChatDetails {
        id: chat_id,
        started_at: chat.2,
        ended_at: chat.3,
        messages,
    }))
}
