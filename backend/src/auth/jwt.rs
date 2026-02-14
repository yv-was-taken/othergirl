use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
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
