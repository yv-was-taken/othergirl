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
    pub iat: i64,
    pub exp: i64,
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
        iat: now.timestamp(),
        exp: expires.timestamp(),
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
    redis: &crate::redis_client::RedisPool,
) -> AppResult<()> {
    let remaining = claims.exp - Utc::now().timestamp();
    if remaining <= 0 {
        return Ok(());
    }

    let key = revocation_key(&claims.jti);
    let mut conn = redis.get().await.map_err(|e| {
        crate::error::AppError::Internal(format!("redis pool error: {e}"))
    })?;
    conn.set_ex::<_, _, ()>(&key, 1u8, remaining as u64).await?;

    Ok(())
}

/// Check whether a token has been revoked by its jti.
pub async fn is_token_revoked(jti: &str, redis: &crate::redis_client::RedisPool) -> AppResult<bool> {
    let key = revocation_key(jti);

    let mut conn = redis.get().await.map_err(|e| {
        crate::error::AppError::Internal(format!("redis pool error: {e}"))
    })?;
    let exists: bool = conn.exists(&key).await?;

    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> JwtSettings {
        JwtSettings::new("a-very-long-test-secret-that-is-at-least-32-chars".to_owned(), 60)
    }

    #[test]
    fn valid_token_decodes_correctly() {
        let settings = test_settings();
        let user_id = Uuid::new_v4();
        let token = issue_token(user_id, &settings).unwrap();
        let claims = verify_token(&token, &settings).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn jti_claim_is_present_and_valid_uuid() {
        let settings = test_settings();
        let user_id = Uuid::new_v4();
        let token = issue_token(user_id, &settings).unwrap();
        let claims = verify_token(&token, &settings).unwrap();
        assert!(!claims.jti.is_empty());
        Uuid::parse_str(&claims.jti).expect("jti should be a valid UUID");
    }

    #[test]
    fn expired_token_fails() {
        let settings = test_settings();
        // Manually craft a token that expired 10 minutes ago (beyond default leeway)
        let now = Utc::now();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            jti: Uuid::new_v4().to_string(),
            iat: (now - Duration::minutes(20)).timestamp(),
            exp: (now - Duration::minutes(10)).timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(settings.secret.as_bytes()),
        )
        .unwrap();
        let result = verify_token(&token, &settings);
        assert!(result.is_err(), "expired token should fail verification");
    }

    #[test]
    fn tampered_token_fails() {
        let settings = test_settings();
        let user_id = Uuid::new_v4();
        let token = issue_token(user_id, &settings).unwrap();

        // Tamper with the token by flipping a character in the signature
        let mut tampered = token.clone();
        let last = tampered.pop().unwrap();
        tampered.push(if last == 'A' { 'B' } else { 'A' });

        let result = verify_token(&tampered, &settings);
        assert!(result.is_err(), "tampered token should fail verification");
    }

    #[test]
    fn wrong_secret_fails() {
        let settings = test_settings();
        let user_id = Uuid::new_v4();
        let token = issue_token(user_id, &settings).unwrap();

        let wrong_settings = JwtSettings::new(
            "a-completely-different-secret-that-is-long-enough".to_owned(),
            60,
        );
        let result = verify_token(&token, &wrong_settings);
        assert!(result.is_err(), "token verified with wrong secret should fail");
    }

    #[test]
    fn different_users_get_different_jtis() {
        let settings = test_settings();
        let token1 = issue_token(Uuid::new_v4(), &settings).unwrap();
        let token2 = issue_token(Uuid::new_v4(), &settings).unwrap();
        let claims1 = verify_token(&token1, &settings).unwrap();
        let claims2 = verify_token(&token2, &settings).unwrap();
        assert_ne!(claims1.jti, claims2.jti);
    }
}
