use axum::{
    extract::{ConnectInfo, State},
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::net::{IpAddr, SocketAddr};

use crate::{
    error::{AppError, AppResult},
    AppState,
};

const WINDOW_SECONDS: u64 = 60;
const MAX_REQUESTS_PER_WINDOW: i64 = 180;
const MAX_AUTH_REQUESTS_PER_WINDOW: i64 = 40;

pub async fn middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> AppResult<Response> {
    let path = request.uri().path().to_owned();

    if !path.starts_with("/api") {
        return Ok(next.run(request).await);
    }

    let source = source_ip_for_request(&request, state.config.trusted_proxy_hops)?.to_string();

    let bucket = if path.starts_with("/api/auth") {
        "auth"
    } else {
        "general"
    };

    let key = format!("rate_limit:{bucket}:{source}");
    let limit = if bucket == "auth" {
        MAX_AUTH_REQUESTS_PER_WINDOW
    } else {
        MAX_REQUESTS_PER_WINDOW
    };

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

fn source_ip_for_request(
    request: &Request<axum::body::Body>,
    trusted_proxy_hops: usize,
) -> AppResult<IpAddr> {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip())
        .ok_or(AppError::Internal)?;

    if trusted_proxy_hops == 0 {
        return Ok(peer_ip);
    }

    let forwarded_for = request
        .headers()
        .get(header::HeaderName::from_static("x-forwarded-for"))
        .and_then(|value| value.to_str().ok());

    let Some(forwarded_for) = forwarded_for else {
        return Ok(peer_ip);
    };

    let Some(chain) = parse_forwarded_for_chain(forwarded_for) else {
        return Ok(peer_ip);
    };

    let index = chain.len().saturating_sub(trusted_proxy_hops);
    Ok(chain[index])
}

fn parse_forwarded_for_chain(raw: &str) -> Option<Vec<IpAddr>> {
    let mut chain = Vec::new();

    for value in raw.split(',') {
        let candidate = value.trim();
        if candidate.is_empty() {
            return None;
        }

        let ip = candidate.parse::<IpAddr>().ok()?;
        chain.push(ip);
    }

    if chain.is_empty() {
        return None;
    }

    Some(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_with_source(
        peer: SocketAddr,
        forwarded_for: Option<&str>,
    ) -> Request<axum::body::Body> {
        let mut builder = Request::builder().uri("/api/test");
        if let Some(forwarded_for) = forwarded_for {
            builder = builder.header("x-forwarded-for", forwarded_for);
        }

        let mut request = builder.body(axum::body::Body::empty()).unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        request
    }

    #[test]
    fn uses_peer_ip_when_trusted_proxy_hops_is_zero() {
        let peer = SocketAddr::from(([10, 0, 0, 1], 12345));
        let request = request_with_source(peer, Some("203.0.113.10"));

        let source = source_ip_for_request(&request, 0).unwrap();

        assert_eq!(source, IpAddr::from([10, 0, 0, 1]));
    }

    #[test]
    fn uses_rightmost_untrusted_hop_from_forwarded_for_chain() {
        let peer = SocketAddr::from(([10, 0, 0, 1], 12345));
        let request = request_with_source(peer, Some("198.51.100.1, 203.0.113.7"));

        let source = source_ip_for_request(&request, 1).unwrap();

        assert_eq!(source, IpAddr::from([203, 0, 113, 7]));
    }

    #[test]
    fn falls_back_to_peer_ip_when_forwarded_for_is_invalid() {
        let peer = SocketAddr::from(([10, 0, 0, 2], 12345));
        let request = request_with_source(peer, Some("198.51.100.1, nope"));

        let source = source_ip_for_request(&request, 1).unwrap();

        assert_eq!(source, IpAddr::from([10, 0, 0, 2]));
    }
}
