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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn status_of(err: AppError) -> StatusCode {
        err.into_response().status()
    }

    #[test]
    fn bad_request_returns_400() {
        assert_eq!(status_of(AppError::BadRequest("test".into())), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unauthorized_returns_401() {
        assert_eq!(status_of(AppError::Unauthorized("test".into())), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_returns_403() {
        assert_eq!(status_of(AppError::Forbidden("test".into())), StatusCode::FORBIDDEN);
    }

    #[test]
    fn not_found_returns_404() {
        assert_eq!(status_of(AppError::NotFound("test".into())), StatusCode::NOT_FOUND);
    }

    #[test]
    fn conflict_returns_409() {
        assert_eq!(status_of(AppError::Conflict("test".into())), StatusCode::CONFLICT);
    }

    #[test]
    fn too_many_requests_returns_429() {
        assert_eq!(status_of(AppError::TooManyRequests("test".into())), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn service_unavailable_returns_503() {
        assert_eq!(status_of(AppError::ServiceUnavailable("test".into())), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn internal_returns_500() {
        assert_eq!(status_of(AppError::Internal("test".into())), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn internal_error_does_not_leak_details() {
        let resp = AppError::Internal("secret-db-error".into()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // The body should say "internal server error", not the actual message.
        // We verify this via the response body.
        let body = tokio::runtime::Runtime::new().unwrap().block_on(async {
            let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        });
        assert!(!body.contains("secret-db-error"));
        assert!(body.contains("internal server error"));
    }

    #[test]
    fn from_jsonwebtoken_error_produces_unauthorized() {
        // Create a jsonwebtoken error by trying to decode garbage
        let err: Result<jsonwebtoken::TokenData<serde_json::Value>, _> =
            jsonwebtoken::decode::<serde_json::Value>(
                "not.a.token",
                &jsonwebtoken::DecodingKey::from_secret(b"secret"),
                &jsonwebtoken::Validation::default(),
            );
        let app_err: AppError = err.unwrap_err().into();
        assert_eq!(status_of(app_err), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn response_body_is_json_with_error_field() {
        let resp = AppError::NotFound("thing missing".into()).into_response();
        let body = tokio::runtime::Runtime::new().unwrap().block_on(async {
            let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        });
        assert!(body.get("error").is_some());
        assert!(body["error"].as_str().unwrap().contains("thing missing"));
    }
}
