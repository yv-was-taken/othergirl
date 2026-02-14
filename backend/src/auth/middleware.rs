use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    auth::jwt::{verify_token, JwtSettings},
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    JwtSettings: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> AppResult<Self> {
        let jwt_settings = JwtSettings::from_ref(state);

        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("missing authorization header".to_owned()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("malformed bearer token".to_owned()))?;

        let claims = verify_token(token, &jwt_settings)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;

        Ok(Self { user_id })
    }
}
