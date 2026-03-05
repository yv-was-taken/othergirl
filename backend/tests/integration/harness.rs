use std::{net::SocketAddr, sync::Arc};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::Rng;
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use othergirl_backend::{
    auth::jwt::JwtSettings, build_app, chat::websocket::ChatHub, config::AppConfig, AppState,
};

/// A running test server with an HTTP client pointed at it.
pub struct TestApp {
    pub addr: SocketAddr,
    pub client: Client,
    pub state: AppState,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    // ---- convenience request helpers ----

    pub async fn get(&self, path: &str) -> Response {
        self.client
            .get(self.url(path))
            .send()
            .await
            .expect("request failed")
    }

    pub async fn post(&self, path: &str, body: &Value) -> Response {
        self.client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .expect("request failed")
    }

    pub async fn authed_get(&self, path: &str, token: &str) -> Response {
        self.client
            .get(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("request failed")
    }

    pub async fn authed_post(&self, path: &str, token: &str, body: &Value) -> Response {
        self.client
            .post(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .expect("request failed")
    }

    pub async fn authed_put(&self, path: &str, token: &str, body: &Value) -> Response {
        self.client
            .put(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .expect("request failed")
    }

    // ---- high-level helpers ----

    /// Register a new user with unique credentials. Returns (token, user_id).
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> (String, Uuid) {
        let resp = self
            .post(
                "/api/auth/register",
                &json!({
                    "username": username,
                    "email": email,
                    "password": password,
                }),
            )
            .await;

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "register_user helper failed: {}",
            resp.status()
        );

        let body: Value = resp.json().await.unwrap();
        let token = body["token"].as_str().unwrap().to_owned();
        let user_id = Uuid::parse_str(body["user"]["id"].as_str().unwrap()).unwrap();
        (token, user_id)
    }

    /// Generate a unique username and email for a test.
    pub fn unique_credentials(&self) -> (String, String) {
        let id = Uuid::new_v4().to_string();
        let short = &id[..8];
        (format!("user_{short}"), format!("{short}@test.local"))
    }

    /// Clean up: truncate user-created data so tests don't leak.
    pub async fn cleanup(&self) {
        sqlx::query(
            "TRUNCATE users, user_interests, chat_sessions, chat_messages, \
             queue_entries, keep_votes, awards, sparks_transactions, \
             sparks_balances, user_flare, cashout_requests, webhook_events \
             CASCADE",
        )
        .execute(&self.state.db)
        .await
        .ok(); // ignore errors from missing tables
    }
}

/// Spin up a test server. Call once per test or share via `once_cell`.
pub async fn spawn_app() -> TestApp {
    let database_url = "postgres://test:test@localhost:5433/othergirl_test";
    let redis_url = "redis://127.0.0.1:6380";

    // Connect to Postgres and run migrations
    let db = othergirl_backend::db::connect(database_url, 5)
        .await
        .expect("failed to connect to test postgres");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    // Connect to Redis
    let redis = othergirl_backend::redis_client::connect(redis_url)
        .expect("failed to connect to test redis");

    // Generate a random 32-byte encryption key
    let mut key_bytes = [0u8; 32];
    rand::rng().fill(&mut key_bytes);
    let _chat_key_b64 = STANDARD.encode(key_bytes);

    let jwt_secret = "integration-test-jwt-secret-that-is-long-enough-1234".to_owned();

    let config = AppConfig {
        server_addr: "127.0.0.1:0".to_owned(),
        database_url: database_url.to_owned(),
        redis_url: redis_url.to_owned(),
        trusted_proxy_hops: 0,
        jwt_secret: jwt_secret.clone(),
        jwt_ttl_minutes: 60,
        cors_origin: "http://localhost:3000".to_owned(),
        public_api_base_url: "http://localhost:8080".to_owned(),
        public_web_base_url: "http://localhost:3000".to_owned(),
        stripe_secret_key: None,
        stripe_webhook_secret: None,
        stripe_premium_price_id: None,
        stripe_success_url: "http://localhost:3000/settings".to_owned(),
        stripe_cancel_url: "http://localhost:3000/settings".to_owned(),
        stripe_connect_refresh_url: "http://localhost:3000/settings".to_owned(),
        stripe_connect_return_url: "http://localhost:3000/settings".to_owned(),
        google_oauth_client_id: None,
        google_oauth_client_secret: None,
        discord_oauth_client_id: None,
        discord_oauth_client_secret: None,
        github_oauth_client_id: None,
        github_oauth_client_secret: None,
        telegram_bot_token: None,
        oauth_state_ttl_seconds: 600,
        matcher_interval_ms: 60000, // very slow so it doesn't interfere
        queue_session_ttl_seconds: 3600,
        cooldown_seconds: 5,
        chat_key_encryption_key: key_bytes,
        legacy_chat_key_encryption_keys: vec![],
        emote_upload_dir: "/tmp/othergirl_test_emotes".to_owned(),
        emote_public_base_url: "http://localhost:8080/assets/emotes".to_owned(),
        admin_user_ids: vec![],
        db_max_connections: 5,
    };

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        redis,
        jwt: JwtSettings::new(config.jwt_secret.clone(), config.jwt_ttl_minutes),
        chat_hub: ChatHub::new(),
    };

    let app = build_app(state.clone(), &config.cors_origin);

    // Bind to a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind test listener");
    let addr = listener.local_addr().unwrap();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("test server failed");
    });

    // Create emote upload directory
    std::fs::create_dir_all("/tmp/othergirl_test_emotes").ok();

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let test_app = TestApp {
        addr,
        client,
        state,
    };

    // Clean up any leftover data from previous test runs
    test_app.cleanup().await;

    // Flush test Redis
    let mut conn = test_app
        .state
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .unwrap();
    let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await.unwrap();

    test_app
}
