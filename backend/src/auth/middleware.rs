use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::jwt::{is_token_revoked, verify_token, JwtSettings},
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: Uuid,
}

pub async fn ensure_account_active(db: &PgPool, user_id: Uuid) -> AppResult<()> {
    let (is_suspended, deletion_due) = sqlx::query_as::<_, (bool, bool)>(
        r#"
        SELECT
            is_suspended,
            COALESCE(deletion_scheduled_at <= NOW(), FALSE) AS deletion_due
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("user not found".to_owned()))?;

    if is_suspended {
        return Err(AppError::Forbidden("account suspended".to_owned()));
    }

    if deletion_due {
        return Err(AppError::Forbidden(
            "account deletion deadline reached".to_owned(),
        ));
    }

    Ok(())
}

impl<S> FromRequestParts<S> for AuthUser
where
    JwtSettings: FromRef<S>,
    crate::redis_client::RedisPool: FromRef<S>,
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> AppResult<Self> {
        let jwt_settings = JwtSettings::from_ref(state);
        let redis = crate::redis_client::RedisPool::from_ref(state);

        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("missing authorization header".to_owned()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("malformed bearer token".to_owned()))?;

        let claims = verify_token(token, &jwt_settings)?;

        if is_token_revoked(&claims.jti, &redis).await? {
            return Err(AppError::Unauthorized("token has been revoked".to_owned()));
        }

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;

        let db = PgPool::from_ref(state);
        ensure_account_active(&db, user_id).await?;

        Ok(Self { user_id })
    }
}
