use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FlareItem {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub item_type: String,
    pub price_sparks: i64,
    pub rarity: String,
    pub asset_data: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct UserFlare {
    pub user_id: Uuid,
    pub flare_item_id: Uuid,
    pub is_equipped: bool,
    pub purchased_at: DateTime<Utc>,
}
