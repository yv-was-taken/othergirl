use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Emote {
    pub id: Uuid,
    pub token: String,
    pub name: String,
    pub image_url: String,
    pub price_sparks: i64,
    pub is_active: bool,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
