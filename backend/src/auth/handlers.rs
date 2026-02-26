use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::{
        jwt::issue_token,
        middleware::AuthUser,
        password::{hash_password, verify_password},
    },
    error::{AppError, AppResult},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub is_age_verified: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: SessionUser,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionUser {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub is_premium: bool,
    pub is_age_verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserAuthRow {
    id: Uuid,
    username: String,
    email: Option<String>,
    password_hash: Option<String>,
    is_premium: bool,
    is_age_verified: bool,
    is_suspended: bool,
    created_at: DateTime<Utc>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    payload.validate()?;

    let password_hash = hash_password(&payload.password)?;
    let is_age_verified = payload.is_age_verified.unwrap_or(false);

    let insert = sqlx::query_as::<_, UserAuthRow>(
        r#"
        INSERT INTO users (username, email, password_hash, is_age_verified)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, password_hash, is_premium, is_age_verified, is_suspended, created_at
        "#,
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(password_hash)
    .bind(is_age_verified)
    .fetch_one(&state.db)
    .await;

    let user = match insert {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            return Err(AppError::Conflict(
                "email or username already exists".to_owned(),
            ))
        }
        Err(err) => return Err(err.into()),
    };

    let token = issue_token(user.id, &state.jwt)?;

    Ok(Json(AuthResponse {
        token,
        user: SessionUser {
            id: user.id,
            username: user.username,
            email: user.email,
            is_premium: user.is_premium,
            is_age_verified: user.is_age_verified,
            created_at: user.created_at,
        },
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    payload.validate()?;

    let user = sqlx::query_as::<_, UserAuthRow>(
        r#"
        SELECT id, username, email, password_hash, is_premium, is_age_verified, is_suspended, created_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(payload.email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid email or password".to_owned()))?;

    if user.is_suspended {
        return Err(AppError::Forbidden("account suspended".to_owned()));
    }

    let hash = user.password_hash.as_deref().ok_or_else(|| {
        AppError::Unauthorized("password login unavailable for this account".to_owned())
    })?;

    if !verify_password(hash, &payload.password)? {
        return Err(AppError::Unauthorized(
            "invalid email or password".to_owned(),
        ));
    }

    let token = issue_token(user.id, &state.jwt)?;

    Ok(Json(AuthResponse {
        token,
        user: SessionUser {
            id: user.id,
            username: user.username,
            email: user.email,
            is_premium: user.is_premium,
            is_age_verified: user.is_age_verified,
            created_at: user.created_at,
        },
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let is_suspended: bool = sqlx::query_scalar(
        "SELECT is_suspended FROM users WHERE id = $1",
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("user not found".to_owned()))?;

    if is_suspended {
        return Err(AppError::Forbidden("account suspended".to_owned()));
    }

    let token = issue_token(auth_user.user_id, &state.jwt)?;

    Ok(Json(serde_json::json!({ "token": token })))
}
