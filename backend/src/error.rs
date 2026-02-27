use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            Self::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            Self::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            Self::Internal(ref msg) => {
                error!(details = %msg, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };

        let body = ErrorBody { error: body };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        Self::Unauthorized("invalid token".to_owned())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::BadRequest(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
