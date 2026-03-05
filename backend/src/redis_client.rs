use std::time::Duration;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;

use crate::error::{AppError, AppResult};

pub type RedisPool = Pool<RedisConnectionManager>;

pub async fn connect(redis_url: &str) -> AppResult<RedisPool> {
    let manager = RedisConnectionManager::new(redis_url)
        .map_err(|e| AppError::Internal(format!("redis connection manager error: {e}")))?;

    let pool = Pool::builder()
        .max_size(10)
        .min_idle(Some(2))
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .await
        .map_err(|e| AppError::Internal(format!("redis pool error: {e}")))?;

    Ok(pool)
}
