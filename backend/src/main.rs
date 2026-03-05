mod auth;
mod awards;
mod categories;
mod chat;
mod config;
mod db;
mod emotes;
mod error;
mod flare;
mod matchmaking;
mod moderation;
mod payments;
mod rate_limit;
mod redis_client;
mod users;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{FromRef, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};

use crate::{auth::jwt::JwtSettings, config::AppConfig};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub redis: ::redis::Client,
    pub jwt: JwtSettings,
    pub chat_hub: chat::websocket::ChatHub,
}

impl FromRef<AppState> for JwtSettings {
    fn from_ref(state: &AppState) -> Self {
        state.jwt.clone()
    }
}

impl FromRef<AppState> for redis::Client {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "othergirl_backend=debug,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let db = db::connect(&config.database_url, config.db_max_connections).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let redis = redis_client::connect(&config.redis_url)?;

    // Validate Redis connection
    let mut conn = redis.get_multiplexed_tokio_connection().await?;
    redis::cmd("PING").query_async::<_, String>(&mut conn).await
        .expect("failed to connect to Redis — is it running?");
    info!("Redis connection validated");

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        redis,
        jwt: JwtSettings::new(config.jwt_secret, config.jwt_ttl_minutes),
        chat_hub: chat::websocket::ChatHub::new(),
    };

    let shutdown_token = tokio_util::sync::CancellationToken::new();

    let matcher_handle = matchmaking::matcher::spawn_matcher(state.clone(), shutdown_token.clone());
    let reconciler_handle = payments::spawn_background_jobs(state.clone(), shutdown_token.clone());

    if config.cors_origin.trim() == "*" {
        panic!(
            "CORS_ORIGIN is set to \"*\" (wildcard). \
             This allows any website to make authenticated requests to the API. \
             Set CORS_ORIGIN to a specific origin, e.g. \"http://localhost:5173\" \
             or \"https://your-domain.com\". Multiple origins can be comma-separated."
        );
    }

    let cors = {
        let origins: Vec<axum::http::HeaderValue> = config
            .cors_origin
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|o| {
                o.parse::<axum::http::HeaderValue>()
                    .unwrap_or_else(|_| panic!("invalid CORS_ORIGIN value: {o:?}"))
            })
            .collect();

        if origins.is_empty() {
            panic!("CORS_ORIGIN must contain at least one origin");
        }

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
        .route("/api/chats/:id", get(chat::handlers::get_chat))
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
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::middleware,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .nest_service(
            "/assets/emotes",
            ServeDir::new(state.config.emote_upload_dir.as_str())
                .append_index_html_on_directories(false),
        )
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
        .with_state(state);

    let addr: SocketAddr = config.server_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("backend listening on {addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("shutdown signal received, stopping background jobs…");
        shutdown_token.cancel();
    })
    .await?;

    info!("HTTP server stopped, waiting for background tasks to finish…");
    let drain = async {
        let _ = matcher_handle.await;
        let _ = reconciler_handle.await;
    };
    if tokio::time::timeout(std::time::Duration::from_secs(5), drain)
        .await
        .is_err()
    {
        warn!("background tasks did not finish within 5 s, forcing exit");
    } else {
        info!("all background tasks exited cleanly");
    }

    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let redis_ok = {
        let mut conn = state.redis.get_multiplexed_tokio_connection().await;
        match conn {
            Ok(ref mut c) => redis::cmd("PING").query_async::<_, String>(c).await.is_ok(),
            Err(_) => false,
        }
    };
    let status = if db_ok && redis_ok { "ok" } else { "unhealthy" };
    Json(serde_json::json!({
        "status": status,
        "db": db_ok,
        "redis": redis_ok,
    }))
}
