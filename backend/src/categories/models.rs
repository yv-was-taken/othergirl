use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub parent_id: Option<Uuid>,
    pub is_nsfw: bool,
    pub active_user_count: i32,
    pub visibility_threshold: i32,
    pub is_visible: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Language {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct SuggestCategoryRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_nsfw: Option<bool>,
}
