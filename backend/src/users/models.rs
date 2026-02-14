use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub bio: String,
    pub is_premium: bool,
    pub is_age_verified: bool,
    pub keep_count: i32,
    pub reputation_score: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 500))]
    pub bio: Option<String>,
    pub is_age_verified: Option<bool>,
    pub interest_category_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFlareRequest {
    pub flare_item_ids: Vec<Uuid>,
}
