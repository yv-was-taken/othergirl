use std::env;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub trusted_proxy_hops: usize,
    pub jwt_secret: String,
    pub jwt_ttl_minutes: i64,
    pub cors_origin: String,
    pub public_api_base_url: String,
    pub public_web_base_url: String,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_premium_price_id: Option<String>,
    pub stripe_success_url: String,
    pub stripe_cancel_url: String,
    pub stripe_connect_refresh_url: String,
    pub stripe_connect_return_url: String,
    pub google_oauth_client_id: Option<String>,
    pub google_oauth_client_secret: Option<String>,
    pub discord_oauth_client_id: Option<String>,
    pub discord_oauth_client_secret: Option<String>,
    pub github_oauth_client_id: Option<String>,
    pub github_oauth_client_secret: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub oauth_state_ttl_seconds: u64,
    pub matcher_interval_ms: u64,
    pub queue_session_ttl_seconds: u64,
    pub cooldown_seconds: u64,
    pub chat_key_encryption_key: [u8; 32],
    pub emote_upload_dir: String,
    pub emote_public_base_url: String,
    pub admin_user_ids: Vec<Uuid>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-now".to_owned());
        let chat_key_encryption_key = chat_key_encryption_key_from_env(&jwt_secret);
        let public_api_base_url =
            env::var("PUBLIC_API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_owned());
        let public_web_base_url =
            env::var("PUBLIC_WEB_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned());

        Self {
            server_addr: env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned()),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/othergirl".to_owned()
            }),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned()),
            trusted_proxy_hops: env_usize("TRUSTED_PROXY_HOPS", 0),
            jwt_secret,
            jwt_ttl_minutes: env::var("JWT_TTL_MINUTES")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60),
            cors_origin: env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_owned()),
            public_api_base_url: public_api_base_url.clone(),
            public_web_base_url: public_web_base_url.clone(),
            stripe_secret_key: optional_env("STRIPE_SECRET_KEY"),
            stripe_webhook_secret: optional_env("STRIPE_WEBHOOK_SECRET"),
            stripe_premium_price_id: optional_env("STRIPE_PREMIUM_PRICE_ID"),
            stripe_success_url: env::var("STRIPE_SUCCESS_URL")
                .unwrap_or_else(|_| format!("{public_web_base_url}/settings")),
            stripe_cancel_url: env::var("STRIPE_CANCEL_URL")
                .unwrap_or_else(|_| format!("{public_web_base_url}/settings")),
            stripe_connect_refresh_url: env::var("STRIPE_CONNECT_REFRESH_URL")
                .unwrap_or_else(|_| format!("{public_web_base_url}/settings")),
            stripe_connect_return_url: env::var("STRIPE_CONNECT_RETURN_URL")
                .unwrap_or_else(|_| format!("{public_web_base_url}/settings")),
            google_oauth_client_id: optional_env("GOOGLE_OAUTH_CLIENT_ID"),
            google_oauth_client_secret: optional_env("GOOGLE_OAUTH_CLIENT_SECRET"),
            discord_oauth_client_id: optional_env("DISCORD_OAUTH_CLIENT_ID"),
            discord_oauth_client_secret: optional_env("DISCORD_OAUTH_CLIENT_SECRET"),
            github_oauth_client_id: optional_env("GITHUB_OAUTH_CLIENT_ID"),
            github_oauth_client_secret: optional_env("GITHUB_OAUTH_CLIENT_SECRET"),
            telegram_bot_token: optional_env("TELEGRAM_BOT_TOKEN"),
            oauth_state_ttl_seconds: env_u64("OAUTH_STATE_TTL_SECONDS", 10 * 60),
            matcher_interval_ms: env_u64("MATCHER_INTERVAL_MS", 500),
            queue_session_ttl_seconds: env_u64("QUEUE_SESSION_TTL_SECONDS", 60 * 60),
            cooldown_seconds: env_u64("MATCH_COOLDOWN_SECONDS", 5),
            chat_key_encryption_key,
            emote_upload_dir: env::var("EMOTE_UPLOAD_DIR")
                .unwrap_or_else(|_| "uploads/emotes".to_owned()),
            emote_public_base_url: env::var("EMOTE_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| format!("{public_api_base_url}/assets/emotes")),
            admin_user_ids: parse_admin_user_ids(),
        }
    }
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn chat_key_encryption_key_from_env(jwt_secret: &str) -> [u8; 32] {
    if let Some(raw) = optional_env("CHAT_KEY_ENCRYPTION_KEY_B64") {
        if let Ok(decoded) = STANDARD.decode(raw.as_bytes()) {
            if decoded.len() == 32 {
                let mut key = [0_u8; 32];
                key.copy_from_slice(&decoded);
                return key;
            }
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(jwt_secret.as_bytes());
    hasher.update(b":chat-key-wrap");
    let digest = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn parse_admin_user_ids() -> Vec<Uuid> {
    env::var("ADMIN_USER_IDS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter_map(|raw| Uuid::parse_str(raw).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
