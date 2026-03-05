pub mod admin;
pub mod auth;
pub mod awards;
pub mod categories;
pub mod chat;
pub mod config;
pub mod db;
pub mod emotes;
pub mod error;
pub mod flare;
pub mod matchmaking;
pub mod metrics;
pub mod moderation;
pub mod notifications;
pub mod payments;
pub mod rate_limit;
pub mod redis_client;
pub mod users;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{FromRef, MatchedPath, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer, services::ServeDir, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};

use crate::{auth::jwt::JwtSettings, config::AppConfig, redis_client::RedisPool};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub redis: RedisPool,
    pub jwt: JwtSettings,
    pub chat_hub: chat::websocket::ChatHub,
}

impl FromRef<AppState> for JwtSettings {
    fn from_ref(state: &AppState) -> Self {
        state.jwt.clone()
    }
}

impl FromRef<AppState> for RedisPool {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

pub fn build_app(state: AppState, cors_origin: &str) -> Router {
    let cors = {
        let origins: Vec<axum::http::HeaderValue> = cors_origin
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|o| {
                o.parse::<axum::http::HeaderValue>()
                    .unwrap_or_else(|_| panic!("invalid CORS_ORIGIN value: {o:?}"))
            })
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };

    let api = Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/categories", categories::routes())
        .route("/api/languages", get(categories::handlers::list_languages))
        .nest("/api/matchmaking", matchmaking::routes())
        .route("/api/chat", get(chat::websocket::ws_handler))
        .route("/api/chats", get(chat::handlers::list_chats))
        .route("/api/chats/keeps", get(chat::handlers::list_kept_chats))
        .route("/api/chats/{id}", get(chat::handlers::get_chat))
        .nest("/api/payments", payments::routes())
        .route("/api/sparks/balance", get(payments::handlers::balance))
        .route(
            "/api/sparks/transactions",
            get(payments::handlers::transactions),
        )
        .route(
            "/api/cashout/connect",
            post(payments::handlers::cashout_connect),
        )
        .route(
            "/api/cashout/request",
            post(payments::handlers::cashout_request),
        )
        .route(
            "/api/cashout/status",
            get(payments::handlers::cashout_status),
        )
        .nest("/api/store", flare::routes())
        .nest("/api/awards", awards::routes())
        .nest("/api", moderation::routes())
        .nest("/api/emotes", emotes::routes())
        .nest("/api/notifications", notifications::routes())
        .nest("/api/admin", admin::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::middleware,
        ));

    let assets = Router::new()
        .nest_service(
            "/assets/avatars",
            ServeDir::new(std::path::Path::new(&state.config.upload_dir).join("avatars"))
                .append_index_html_on_directories(false),
        )
        .nest_service(
            "/assets/emotes",
            ServeDir::new(state.config.emote_upload_dir.as_str())
                .append_index_html_on_directories(false),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static(
                "public, max-age=86400, stale-while-revalidate=604800",
            ),
        ));

    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .merge(assets)
        .merge(api)
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            axum::http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            axum::http::HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            axum::http::HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(metrics_middleware))
        .with_state(state)
}

async fn metrics_middleware(
    matched_path: Option<MatchedPath>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = matched_path
        .map(|path| path.as_str().to_owned())
        .unwrap_or_else(|| "__unmatched__".to_owned());
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    let duration = start.elapsed().as_secs_f64();

    metrics::HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    metrics::HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &path])
        .observe(duration);

    response
}

async fn metrics_handler() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics::render_metrics(),
    )
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let redis_ok = match state.redis.get().await {
        Ok(mut conn) => redis::cmd("PING")
            .query_async::<String>(&mut *conn)
            .await
            .is_ok(),
        Err(_) => false,
    };
    let status = if db_ok && redis_ok { "ok" } else { "unhealthy" };
    Json(serde_json::json!({
        "status": status,
        "db": db_ok,
        "redis": redis_ok,
    }))
}
