use std::time::Duration;

use rand::seq::SliceRandom;
use redis::AsyncCommands;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    matchmaking::queue::{self, MatchNotification},
    AppState,
};

pub fn spawn_matcher(
    state: AppState,
    cancel: tokio_util::sync::CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(state.config.matcher_interval_ms));

        loop {
            tokio::select! {
                _ = ticker.tick() => {}
                _ = cancel.cancelled() => {
                    info!("matcher received shutdown signal, exiting");
                    break;
                }
            }
            if let Err(err) = run_match_cycle(&state).await {
                error!(?err, "matcher cycle failed");
            }
        }
    })
}

async fn run_match_cycle(state: &AppState) -> Result<(), crate::error::AppError> {
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let queue_keys = queue::queue_keys(&mut conn).await?;

    for queue_key in queue_keys {
        try_match_queue(state, &mut conn, queue_key.as_str()).await?;
    }

    Ok(())
}

async fn try_match_queue(
    state: &AppState,
    conn: &mut redis::aio::MultiplexedConnection,
    queue_key: &str,
) -> Result<(), crate::error::AppError> {
    loop {
        let mut users = queue::queued_users(conn, queue_key, 50).await?;
        if users.len() < 2 {
            return Ok(());
        }

        // Shuffle so every user gets a fair chance at matching,
        // not just those at the front of the sorted set.
        users.shuffle(&mut rand::rng());

        const MAX_PAIR_CHECKS: usize = 50;
        let mut pairs_checked: usize = 0;
        let mut chosen_pair: Option<(Uuid, Uuid)> = None;

        'outer: for (left_idx, left_user) in users.iter().enumerate() {
            if queue::active_chat_for(conn, *left_user).await?.is_some() {
                queue::remove_from_all_queues(conn, *left_user).await?;
                continue;
            }

            if !queue::is_user_queued(conn, *left_user).await? {
                queue::remove_from_queue(conn, queue_key, *left_user).await?;
                continue;
            }

            for right_user in users.iter().skip(left_idx + 1) {
                if pairs_checked >= MAX_PAIR_CHECKS {
                    break 'outer;
                }
                pairs_checked += 1;

                if queue::active_chat_for(conn, *right_user).await?.is_some() {
                    queue::remove_from_all_queues(conn, *right_user).await?;
                    continue;
                }

                if !queue::is_user_queued(conn, *right_user).await? {
                    queue::remove_from_queue(conn, queue_key, *right_user).await?;
                    continue;
                }

                if queue::is_match_compatible(state, *left_user, *right_user).await? {
                    chosen_pair = Some((*left_user, *right_user));
                    break 'outer;
                }
            }
        }

        let Some((user_a, user_b)) = chosen_pair else {
            debug!(queue = %queue_key, "no compatible pair found in queue");
            return Ok(());
        };

        let meta_a = queue::read_queue_meta(conn, user_a).await?;
        queue::remove_from_all_queues(conn, user_a).await?;
        queue::remove_from_all_queues(conn, user_b).await?;

        let (category_id, language_id) = if let Some(meta) = meta_a {
            (meta.category_id, meta.language_id)
        } else {
            (None, None)
        };

        let chat_id =
            queue::create_chat_for_users(state, user_a, user_b, category_id, language_id).await?;
        queue::set_active_chat(state, conn, user_a, user_b, chat_id).await?;

        queue::set_match_notification(
            state,
            conn,
            user_a,
            &MatchNotification {
                chat_id,
                partner_id: user_b,
            },
        )
        .await?;
        queue::set_match_notification(
            state,
            conn,
            user_b,
            &MatchNotification {
                chat_id,
                partner_id: user_a,
            },
        )
        .await?;

        let _: i64 = conn
            .publish(
                "matchmaking:matched",
                serde_json::json!({
                    "chat_id": chat_id,
                    "user_a_id": user_a,
                    "user_b_id": user_b
                })
                .to_string(),
            )
            .await
            .unwrap_or(0);
    }
}
