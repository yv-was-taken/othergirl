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
    extract::FromRef,
    middleware,
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::info;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "othergirl_backend=debug,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let db = db::connect(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let redis = redis_client::connect(&config.redis_url)?;

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        redis,
        jwt: JwtSettings::new(config.jwt_secret, config.jwt_ttl_minutes),
        chat_hub: chat::websocket::ChatHub::new(),
    };

    let shutdown_token = tokio_util::sync::CancellationToken::new();

    matchmaking::matcher::spawn_matcher(state.clone());
    payments::spawn_background_jobs(state.clone(), shutdown_token.clone());

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
            ServeDir::new(state.config.emote_upload_dir.as_str()),
        )
        .merge(api)
        .layer(cors)
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

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "othergirl-backend",
        "time": chrono::Utc::now().to_rfc3339(),
    }))
}
