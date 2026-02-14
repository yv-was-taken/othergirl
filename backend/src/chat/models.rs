use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct ChatSummary {
    pub id: Uuid,
    pub partner_id: Uuid,
    pub partner_username: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ChatDetails {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub messages: Vec<ChatMessage>,
}
