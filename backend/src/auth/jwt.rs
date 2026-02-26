use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub jti: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct JwtSettings {
    pub secret: Arc<String>,
    pub ttl_minutes: i64,
}

impl JwtSettings {
    pub fn new(secret: String, ttl_minutes: i64) -> Self {
        Self {
            secret: Arc::new(secret),
            ttl_minutes,
        }
    }
}

pub fn issue_token(user_id: Uuid, settings: &JwtSettings) -> AppResult<String> {
    let now = Utc::now();
    let expires = now + Duration::minutes(settings.ttl_minutes);

    let claims = Claims {
        sub: user_id.to_string(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as usize,
        exp: expires.timestamp() as usize,
    };

    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secret.as_bytes()),
    )?)
}

pub fn verify_token(token: &str, settings: &JwtSettings) -> AppResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(data.claims)
}

fn revocation_key(jti: &str) -> String {
    format!("revoked:{jti}")
}

/// Store a revoked token's jti in Redis with TTL matching the JWT's remaining lifetime.
pub async fn revoke_token(
    claims: &Claims,
    redis: &redis::Client,
) -> AppResult<()> {
    let remaining = claims.exp as i64 - Utc::now().timestamp();
    if remaining <= 0 {
        return Ok(());
    }

    let key = revocation_key(&claims.jti);
    let mut conn = redis.get_multiplexed_tokio_connection().await?;
    conn.set_ex::<_, _, ()>(&key, 1u8, remaining as u64).await?;

    Ok(())
}

/// Check whether a token has been revoked by its jti.
pub async fn is_token_revoked(jti: &str, redis: &redis::Client) -> AppResult<bool> {
    let key = revocation_key(jti);

    let mut conn = redis.get_multiplexed_tokio_connection().await?;
    let exists: bool = conn.exists(&key).await?;

    Ok(exists)
}
