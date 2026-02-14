use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use crate::{error::AppResult, AppState};

static BLOCKLIST: &[&str] = &[
    "csam",
    "minor nudes",
    "racial_slur_placeholder",
    "kill yourself",
];

#[derive(Debug, Clone, Serialize)]
pub struct SafetyScanResult {
    pub flagged: bool,
    pub reasons: Vec<String>,
}

pub async fn scan_message(
    state: &AppState,
    chat_id: Uuid,
    user_id: Uuid,
    content: &str,
) -> AppResult<SafetyScanResult> {
    let mut reasons = keyword_flags(content);

    if is_flooding(state, chat_id, user_id).await? {
        reasons.push("rate:flooding".to_owned());
    }

    if contains_link(content) {
        reasons.push("url:present".to_owned());
    }

    Ok(SafetyScanResult {
        flagged: !reasons.is_empty(),
        reasons,
    })
}

fn keyword_flags(content: &str) -> Vec<String> {
    let text = content.to_ascii_lowercase();

    let mut reasons = Vec::new();
    for term in BLOCKLIST {
        if text.contains(term) {
            reasons.push(format!("keyword:{term}"));
        }
    }

    reasons
}

async fn is_flooding(state: &AppState, chat_id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let key = format!("msg_rate:{chat_id}:{user_id}");

    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let count: i64 = conn.incr(&key, 1_i64).await?;
    if count == 1 {
        let _: bool = conn.expire(&key, 5_i64).await?;
    }

    Ok(count > 10)
}

fn contains_link(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    lower.contains("http://") || lower.contains("https://") || lower.contains("www.")
}
