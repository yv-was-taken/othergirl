use chrono::Utc;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

const LOW_REPUTATION_THRESHOLD: f64 = 0.3;

#[derive(Debug, Clone)]
pub struct QueueJoinTarget {
    pub queue_keys: Vec<String>,
    pub category_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTargetMetadata {
    pub category_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum QueueStatus {
    Idle,
    Queued {
        queue: String,
        position: usize,
        estimated_wait_seconds: u64,
    },
    Matched {
        chat_id: Uuid,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum JoinOutcome {
    Queued {
        queue: String,
        position: usize,
        estimated_wait_seconds: u64,
    },
    Matched {
        chat_id: Uuid,
        partner_id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchNotification {
    pub chat_id: Uuid,
    pub partner_id: Uuid,
}

pub async fn join_or_match(
    state: &AppState,
    user_id: Uuid,
    target: QueueJoinTarget,
) -> AppResult<JoinOutcome> {
    if target.queue_keys.is_empty() {
        return Err(AppError::BadRequest(
            "no queue keys were provided".to_owned(),
        ));
    }

    ensure_user_matchable(state, user_id).await?;

    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;

    if let Some(chat_id) = active_chat_for(&mut conn, user_id).await? {
        let partner_id = resolve_partner_for_chat(state, chat_id, user_id)
            .await?
            .unwrap_or(user_id);

        return Ok(JoinOutcome::Matched {
            chat_id,
            partner_id,
        });
    }

    if in_cooldown(&mut conn, user_id).await? {
        let ttl = cooldown_ttl(&mut conn, user_id).await?;
        return Err(AppError::Conflict(format!(
            "cooldown active, try again in {ttl}s"
        )));
    }

    if let Some(existing_queue_keys) = queued_in(&mut conn, user_id).await? {
        let queue = existing_queue_keys
            .first()
            .cloned()
            .unwrap_or_else(|| target.queue_keys[0].clone());
        let position = queue_position(&mut conn, &queue, user_id)
            .await?
            .unwrap_or(1);
        return Ok(JoinOutcome::Queued {
            queue,
            position,
            estimated_wait_seconds: estimate_wait(position),
        });
    }

    let score = queue_score(state, user_id).await?;
    let ttl = state.config.queue_session_ttl_seconds;

    for queue_key in &target.queue_keys {
        let _: usize = conn.zadd(queue_key, user_id.to_string(), score).await?;
    }

    let _: () = conn
        .set_ex(queue_user_key(user_id), target.queue_keys.join("|"), ttl)
        .await?;
    let _: () = conn
        .set_ex(
            queue_meta_key(user_id),
            serde_json::to_string(&QueueTargetMetadata {
                category_id: target.category_id,
                language_id: target.language_id,
            })
            .map_err(|_| AppError::Internal)?,
            ttl,
        )
        .await?;

    let primary_queue = target.queue_keys[0].clone();
    let position = queue_position(&mut conn, &primary_queue, user_id)
        .await?
        .unwrap_or(1);

    Ok(JoinOutcome::Queued {
        queue: primary_queue,
        position,
        estimated_wait_seconds: estimate_wait(position),
    })
}

pub async fn leave_queue(state: &AppState, user_id: Uuid) -> AppResult<bool> {
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let queue_keys = queued_in(&mut conn, user_id).await?;

    if queue_keys.is_none() {
        return Ok(false);
    }

    remove_from_all_queues(&mut conn, user_id).await?;
    Ok(true)
}

pub async fn get_status(state: &AppState, user_id: Uuid) -> AppResult<QueueStatus> {
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;

    if let Some(chat_id) = active_chat_for(&mut conn, user_id).await? {
        return Ok(QueueStatus::Matched { chat_id });
    }

    if let Some(queue_keys) = queued_in(&mut conn, user_id).await? {
        let primary_queue = queue_keys.first().cloned().unwrap_or_default();
        let position = queue_position(&mut conn, &primary_queue, user_id)
            .await?
            .unwrap_or(1);

        return Ok(QueueStatus::Queued {
            queue: primary_queue,
            position,
            estimated_wait_seconds: estimate_wait(position),
        });
    }

    Ok(QueueStatus::Idle)
}

pub async fn end_chat_session(state: &AppState, user_a_id: Uuid, user_b_id: Uuid) -> AppResult<()> {
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let cooldown = state.config.cooldown_seconds;

    let _: usize = conn.del(active_chat_key(user_a_id)).await?;
    let _: usize = conn.del(active_chat_key(user_b_id)).await?;
    let _: () = conn.set_ex(cooldown_key(user_a_id), "1", cooldown).await?;
    let _: () = conn.set_ex(cooldown_key(user_b_id), "1", cooldown).await?;

    Ok(())
}

pub async fn queue_keys(conn: &mut redis::aio::MultiplexedConnection) -> AppResult<Vec<String>> {
    let mut keys: Vec<String> = conn.keys("queue:*").await?;
    keys.sort_by(|left, right| {
        right
            .matches(':')
            .count()
            .cmp(&left.matches(':').count())
            .then_with(|| left.cmp(right))
    });
    Ok(keys)
}

pub async fn queued_users(
    conn: &mut redis::aio::MultiplexedConnection,
    queue_key: &str,
    limit: isize,
) -> AppResult<Vec<Uuid>> {
    let raw: Vec<String> = conn.zrange(queue_key, 0, limit).await?;
    Ok(raw
        .into_iter()
        .filter_map(|value| Uuid::parse_str(value.as_str()).ok())
        .collect())
}

pub async fn remove_from_queue(
    conn: &mut redis::aio::MultiplexedConnection,
    queue_key: &str,
    user_id: Uuid,
) -> AppResult<()> {
    let _: usize = conn.zrem(queue_key, user_id.to_string()).await?;
    Ok(())
}

pub async fn remove_from_all_queues(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<()> {
    if let Some(queue_keys) = queued_in(conn, user_id).await? {
        for queue_key in queue_keys {
            let _: usize = conn.zrem(&queue_key, user_id.to_string()).await?;
        }
    }

    let _: usize = conn.del(queue_user_key(user_id)).await?;
    let _: usize = conn.del(queue_meta_key(user_id)).await?;
    Ok(())
}

pub async fn set_active_chat(
    state: &AppState,
    conn: &mut redis::aio::MultiplexedConnection,
    user_a_id: Uuid,
    user_b_id: Uuid,
    chat_id: Uuid,
) -> AppResult<()> {
    let ttl = state.config.queue_session_ttl_seconds;
    let _: () = conn
        .set_ex(active_chat_key(user_a_id), chat_id.to_string(), ttl)
        .await?;
    let _: () = conn
        .set_ex(active_chat_key(user_b_id), chat_id.to_string(), ttl)
        .await?;
    Ok(())
}

pub async fn active_chat_for(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<Option<Uuid>> {
    let value: Option<String> = conn.get(active_chat_key(user_id)).await?;
    Ok(value.and_then(|raw| Uuid::parse_str(raw.as_str()).ok()))
}

pub async fn set_match_notification(
    state: &AppState,
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
    notification: &MatchNotification,
) -> AppResult<()> {
    let payload = serde_json::to_string(notification).map_err(|_| AppError::Internal)?;
    let _: () = conn
        .set_ex(
            match_notification_key(user_id),
            payload,
            state.config.queue_session_ttl_seconds,
        )
        .await?;
    Ok(())
}

pub async fn take_match_notification(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Option<MatchNotification>> {
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let payload: Option<String> = conn.get(match_notification_key(user_id)).await?;
    if let Some(payload) = payload {
        let _: usize = conn.del(match_notification_key(user_id)).await?;
        return serde_json::from_str(payload.as_str())
            .map(Some)
            .map_err(|_| AppError::Internal);
    }
    Ok(None)
}

pub async fn read_queue_meta(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<Option<QueueTargetMetadata>> {
    let payload: Option<String> = conn.get(queue_meta_key(user_id)).await?;
    if let Some(payload) = payload {
        return serde_json::from_str(payload.as_str())
            .map(Some)
            .map_err(|_| AppError::Internal);
    }
    Ok(None)
}

pub async fn is_user_queued(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<bool> {
    Ok(conn
        .get::<_, Option<String>>(queue_user_key(user_id))
        .await?
        .is_some())
}

pub async fn is_match_compatible(
    state: &AppState,
    user_id: Uuid,
    candidate_id: Uuid,
) -> AppResult<bool> {
    if user_id == candidate_id {
        return Ok(false);
    }

    let blocked = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM blocks
            WHERE (blocker_id = $1 AND blocked_id = $2)
               OR (blocker_id = $2 AND blocked_id = $1)
        )
        "#,
    )
    .bind(user_id)
    .bind(candidate_id)
    .fetch_one(&state.db)
    .await?;

    if blocked {
        return Ok(false);
    }

    let user_a = sqlx::query_as::<_, (f64, bool)>(
        r#"
        SELECT reputation_score, is_suspended
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    let user_b = sqlx::query_as::<_, (f64, bool)>(
        r#"
        SELECT reputation_score, is_suspended
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(candidate_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(user_a) = user_a else {
        return Ok(false);
    };
    let Some(user_b) = user_b else {
        return Ok(false);
    };

    if user_a.1 || user_b.1 {
        return Ok(false);
    }

    let low_pool_a = user_a.0 < LOW_REPUTATION_THRESHOLD;
    let low_pool_b = user_b.0 < LOW_REPUTATION_THRESHOLD;
    if low_pool_a != low_pool_b {
        return Ok(false);
    }

    Ok(true)
}

pub async fn create_chat_for_users(
    state: &AppState,
    user_a_id: Uuid,
    user_b_id: Uuid,
    category_id: Option<Uuid>,
    language_id: Option<Uuid>,
) -> AppResult<Uuid> {
    let chat_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO chats (user_a_id, user_b_id, category_id, language_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(user_a_id)
    .bind(user_b_id)
    .bind(category_id)
    .bind(language_id)
    .fetch_one(&state.db)
    .await?;

    Ok(chat_id)
}

pub async fn resolve_partner_for_chat(
    state: &AppState,
    chat_id: Uuid,
    requester_id: Uuid,
) -> AppResult<Option<Uuid>> {
    let participants = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT user_a_id, user_b_id
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(participants.map(|(a, b)| if a == requester_id { b } else { a }))
}

async fn ensure_user_matchable(state: &AppState, user_id: Uuid) -> AppResult<()> {
    let suspended = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT is_suspended
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    if suspended {
        return Err(AppError::Forbidden("account suspended".to_owned()));
    }

    Ok(())
}

async fn queue_position(
    conn: &mut redis::aio::MultiplexedConnection,
    queue_key: &str,
    user_id: Uuid,
) -> AppResult<Option<usize>> {
    let rank: Option<isize> = conn.zrank(queue_key, user_id.to_string()).await?;
    Ok(rank.map(|idx| idx as usize + 1))
}

async fn queued_in(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<Option<Vec<String>>> {
    let raw: Option<String> = conn.get(queue_user_key(user_id)).await?;
    Ok(raw.map(|keys| {
        keys.split('|')
            .map(|part| part.trim().to_owned())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    }))
}

async fn queue_score(state: &AppState, user_id: Uuid) -> AppResult<f64> {
    let is_premium = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT is_premium
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    let now = Utc::now().timestamp_millis() as f64;
    let premium_boost = if is_premium { 1_000.0 } else { 0.0 };
    Ok(now - premium_boost)
}

async fn in_cooldown(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<bool> {
    Ok(conn
        .get::<_, Option<String>>(cooldown_key(user_id))
        .await?
        .is_some())
}

async fn cooldown_ttl(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: Uuid,
) -> AppResult<u64> {
    let ttl: i64 = conn.ttl(cooldown_key(user_id)).await.unwrap_or_default();
    Ok(ttl.max(0) as u64)
}

fn estimate_wait(position: usize) -> u64 {
    ((position as u64).saturating_sub(1) * 4).max(4)
}

fn active_chat_key(user_id: Uuid) -> String {
    format!("active_chat:{user_id}")
}

fn queue_user_key(user_id: Uuid) -> String {
    format!("queue_user:{user_id}")
}

fn queue_meta_key(user_id: Uuid) -> String {
    format!("queue_meta:{user_id}")
}

fn cooldown_key(user_id: Uuid) -> String {
    format!("cooldown:{user_id}:match")
}

fn match_notification_key(user_id: Uuid) -> String {
    format!("match_notification:{user_id}")
}
