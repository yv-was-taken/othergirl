use axum::{extract::State, Json};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
    matchmaking::queue::{self, JoinOutcome, QueueJoinTarget, QueueStatus},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct JoinQueueRequest {
    pub category_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
}

pub async fn join_queue(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<JoinQueueRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let target = resolve_queue_target(&state, &payload).await?;
    let outcome = queue::join_or_match(&state, auth_user.user_id, target).await?;

    match outcome {
        JoinOutcome::Queued {
            queue,
            position,
            estimated_wait_seconds,
        } => Ok(Json(serde_json::json!({
            "result": "queued",
            "position": position,
            "estimated_wait_seconds": estimated_wait_seconds,
            "queue": queue
        }))),
        JoinOutcome::Matched {
            chat_id,
            partner_id,
        } => {
            let partner = resolve_partner(&state, chat_id, auth_user.user_id, partner_id).await?;

            Ok(Json(serde_json::json!({
                "result": "matched",
                "chat_id": chat_id,
                "partner": partner
            })))
        }
    }
}

pub async fn leave_queue(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let removed = queue::leave_queue(&state, auth_user.user_id).await?;

    Ok(Json(serde_json::json!({
        "removed": removed
    })))
}

pub async fn queue_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<QueueStatus>> {
    let status = queue::get_status(&state, auth_user.user_id).await?;
    Ok(Json(status))
}

async fn resolve_queue_target(
    state: &AppState,
    payload: &JoinQueueRequest,
) -> AppResult<QueueJoinTarget> {
    let language = if let Some(language_id) = payload.language_id {
        sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT id, code
            FROM languages
            WHERE id = $1 AND is_active = TRUE
            "#,
        )
        .bind(language_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("language is invalid or inactive".to_owned()))?
    } else {
        sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT id, code
            FROM languages
            WHERE code = 'en' AND is_active = TRUE
            LIMIT 1
            "#,
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("default language not found".to_owned()))?
    };

    let category = if let Some(category_id) = payload.category_id {
        sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
            r#"
            SELECT id, slug, parent_id
            FROM categories
            WHERE id = $1 AND is_visible = TRUE
            "#,
        )
        .bind(category_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("category is invalid or hidden".to_owned()))?
    } else {
        sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
            r#"
            SELECT id, slug, parent_id
            FROM categories
            WHERE slug = 'casual'
            LIMIT 1
            "#,
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("default category not found".to_owned()))?
    };

    let mut queue_keys = vec![format!("queue:{}:{}", language.1, category.1)];

    if let Some(parent_id) = category.2 {
        if let Some(parent_slug) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT slug
            FROM categories
            WHERE id = $1
            "#,
        )
        .bind(parent_id)
        .fetch_optional(&state.db)
        .await?
        {
            let broad_queue = format!("queue:{}:{}", language.1, parent_slug);
            if !queue_keys.iter().any(|key| key == &broad_queue) {
                queue_keys.push(broad_queue);
            }
        }
    }

    Ok(QueueJoinTarget {
        queue_keys,
        category_id: Some(category.0),
        language_id: Some(language.0),
    })
}

async fn resolve_partner(
    state: &AppState,
    chat_id: Uuid,
    requester_id: Uuid,
    fallback_partner_id: Uuid,
) -> AppResult<serde_json::Value> {
    let record = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT user_a_id, user_b_id
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(&state.db)
    .await?;

    let partner_id = record
        .map(|(a, b)| if a == requester_id { b } else { a })
        .unwrap_or(fallback_partner_id);

    let partner = sqlx::query_as::<_, (Uuid, String, String, bool)>(
        r#"
        SELECT id, username, bio, is_premium
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(partner_id)
    .fetch_optional(&state.db)
    .await?;

    let interest_slugs = sqlx::query_scalar::<_, String>(
        r#"
        SELECT c.slug
        FROM user_interests ui
        JOIN categories c ON c.id = ui.category_id
        WHERE ui.user_id = $1
        ORDER BY c.slug
        "#,
    )
    .bind(partner_id)
    .fetch_all(&state.db)
    .await?;

    let flare = sqlx::query_as::<_, (String, serde_json::Value)>(
        r#"
        SELECT fi.item_type, fi.asset_data
        FROM user_flare uf
        JOIN flare_items fi ON fi.id = uf.flare_item_id
        WHERE uf.user_id = $1
          AND uf.is_equipped = TRUE
        "#,
    )
    .bind(partner_id)
    .fetch_all(&state.db)
    .await?;

    let flare_map = flare.into_iter().fold(
        serde_json::Map::new(),
        |mut acc, (item_type, asset_data)| {
            acc.insert(item_type, asset_data);
            acc
        },
    );

    if let Some((id, username, bio, is_premium)) = partner {
        let badges = if is_premium { vec!["premium"] } else { vec![] };

        return Ok(serde_json::json!({
            "id": id,
            "username": username,
            "bio": bio,
            "badges": badges,
            "interests": interest_slugs,
            "flare": flare_map
        }));
    }

    Ok(serde_json::json!({
        "id": partner_id,
        "username": "stranger",
        "bio": "",
        "badges": [],
        "interests": [],
        "flare": {}
    }))
}
