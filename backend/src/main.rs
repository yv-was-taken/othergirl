use std::{net::SocketAddr, sync::Arc};

use tracing::{info, warn};

use othergirl_backend::{
    auth::jwt::JwtSettings,
    build_app,
    chat,
    config::AppConfig,
    db,
    matchmaking,
    metrics,
    payments,
    redis_client,
    AppState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "othergirl_backend=debug,tower_http=info".into()),
        )
        .init();

    // Initialize Sentry if DSN is set (keeps _guard alive for entire process)
    let _sentry_guard = std::env::var("SENTRY_DSN").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });

    let config = AppConfig::from_env();
    let db = db::connect(&config.database_url, config.db_max_connections).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let redis = redis_client::connect(&config.redis_url).await?;

    // Validate Redis connection
    {
        let mut conn = redis.get().await.expect("failed to get Redis connection from pool");
        redis::cmd("PING")
            .query_async::<String>(&mut *conn)
            .await
            .expect("failed to connect to Redis — is it running?");
    }
    info!("Redis connection validated");

    let chat_hub = chat::websocket::ChatHub::with_redis(redis.clone(), &config.redis_url);

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        redis,
        jwt: JwtSettings::new(config.jwt_secret, config.jwt_ttl_minutes),
        chat_hub,
    };

    let shutdown_token = tokio_util::sync::CancellationToken::new();

    let matcher_handle =
        matchmaking::matcher::spawn_matcher(state.clone(), shutdown_token.clone());
    let reconciler_handle =
        payments::spawn_background_jobs(state.clone(), shutdown_token.clone());

    if config.cors_origin.trim() == "*" {
        panic!(
            "CORS_ORIGIN is set to \"*\" (wildcard). \
             This allows any website to make authenticated requests to the API. \
             Set CORS_ORIGIN to a specific origin, e.g. \"http://localhost:5173\" \
             or \"https://your-domain.com\". Multiple origins can be comma-separated."
        );
    }

    metrics::init_metrics();

    // Seed the registered-users gauge from the database.
    let (user_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    metrics::REGISTERED_USERS_TOTAL.set(user_count);

    let app = build_app(state, &config.cors_origin);

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
