use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Award {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub chat_id: Uuid,
    pub award_type: String,
    pub spark_amount: i64,
    pub recipient_cut: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendAwardRequest {
    pub chat_id: Uuid,
    pub award_type: String,
    #[validate(range(min = 1))]
    pub spark_amount: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AwardsPagination {
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
}
