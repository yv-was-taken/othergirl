use axum::{
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

const WINDOW_SECONDS: u64 = 60;
const MAX_REQUESTS_PER_WINDOW: i64 = 180;
const MAX_AUTH_REQUESTS_PER_WINDOW: i64 = 40;
const MAX_CASHOUT_REQUESTS_PER_WINDOW: i64 = 5;

pub async fn middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> AppResult<Response> {
    let path = request.uri().path().to_owned();

    if !path.starts_with("/api") {
        return Ok(next.run(request).await);
    }

    let source = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let (bucket, limit) = if path.starts_with("/api/auth") {
        ("auth", MAX_AUTH_REQUESTS_PER_WINDOW)
    } else if path.starts_with("/api/cashout") {
        ("cashout", MAX_CASHOUT_REQUESTS_PER_WINDOW)
    } else {
        ("general", MAX_REQUESTS_PER_WINDOW)
    };

    let key = format!("rate_limit:{bucket}:{source}");

    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let count: i64 = conn.incr(&key, 1_i64).await?;

    if count == 1 {
        let _: bool = conn.expire(&key, WINDOW_SECONDS as i64).await?;
    }

    if count > limit {
        let retry_after: i64 = conn.ttl(&key).await.unwrap_or(1_i64);
        return Err(AppError::TooManyRequests(format!(
            "rate limit exceeded, retry in {retry_after}s"
        )));
    }

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::HeaderName::from_static("x-ratelimit-limit"),
        header::HeaderValue::from_str(&limit.to_string())
            .unwrap_or(header::HeaderValue::from_static("0")),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-ratelimit-remaining"),
        header::HeaderValue::from_str(&(limit - count).max(0).to_string())
            .unwrap_or(header::HeaderValue::from_static("0")),
    );

    Ok(response)
}
