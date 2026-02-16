use std::env;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MIN_JWT_SECRET_LENGTH: usize = 32;
const INSECURE_JWT_SECRETS: &[&str] = &["change-me-now", "replace-with-long-random-secret"];

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_addr: String,
    pub database_url: String,
    pub redis_url: String,
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
    pub legacy_chat_key_encryption_keys: Vec<[u8; 32]>,
    pub emote_upload_dir: String,
    pub emote_public_base_url: String,
    pub admin_user_ids: Vec<Uuid>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let jwt_secret = jwt_secret_from_env();
        let chat_key_encryption_key = chat_key_encryption_key_from_env(&jwt_secret);
        let legacy_chat_key_encryption_keys =
            legacy_chat_key_encryption_keys_from_env(&jwt_secret, &chat_key_encryption_key);
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
            legacy_chat_key_encryption_keys,
            emote_upload_dir: env::var("EMOTE_UPLOAD_DIR")
                .unwrap_or_else(|_| "uploads/emotes".to_owned()),
            emote_public_base_url: env::var("EMOTE_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| format!("{public_api_base_url}/assets/emotes")),
            admin_user_ids: parse_admin_user_ids(),
        }
    }
}

fn jwt_secret_from_env() -> String {
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let jwt_secret = jwt_secret.trim().to_owned();

    if jwt_secret.len() < MIN_JWT_SECRET_LENGTH {
        panic!(
            "JWT_SECRET must be at least {MIN_JWT_SECRET_LENGTH} characters long; got {}",
            jwt_secret.len()
        );
    }

    if INSECURE_JWT_SECRETS
        .iter()
        .any(|placeholder| jwt_secret.eq_ignore_ascii_case(placeholder))
    {
        panic!("JWT_SECRET uses insecure placeholder value");
    }

    jwt_secret
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

fn chat_key_encryption_key_from_env(jwt_secret: &str) -> [u8; 32] {
    if let Some(raw) = optional_env("CHAT_KEY_ENCRYPTION_KEY_B64") {
        return parse_chat_key(raw.as_str(), "CHAT_KEY_ENCRYPTION_KEY_B64");
    }

    derive_chat_key_encryption_key(jwt_secret)
}

fn legacy_chat_key_encryption_keys_from_env(
    jwt_secret: &str,
    current_key: &[u8; 32],
) -> Vec<[u8; 32]> {
    let mut keys = optional_env("CHAT_KEY_ENCRYPTION_KEY_LEGACY_B64")
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| parse_chat_key(value, "CHAT_KEY_ENCRYPTION_KEY_LEGACY_B64"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for legacy_secret in INSECURE_JWT_SECRETS {
        if !jwt_secret.eq_ignore_ascii_case(legacy_secret) {
            keys.push(derive_chat_key_encryption_key(legacy_secret));
        }
    }

    dedupe_chat_keys(keys, current_key)
}

fn dedupe_chat_keys(keys: Vec<[u8; 32]>, current_key: &[u8; 32]) -> Vec<[u8; 32]> {
    let mut deduped = Vec::new();
    for key in keys {
        if &key == current_key {
            continue;
        }
        if deduped.iter().any(|existing| existing == &key) {
            continue;
        }
        deduped.push(key);
    }
    deduped
}

fn derive_chat_key_encryption_key(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b":chat-key-wrap");
    let digest = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn parse_chat_key(raw: &str, env_name: &str) -> [u8; 32] {
    let decoded = STANDARD
        .decode(raw.as_bytes())
        .unwrap_or_else(|_| panic!("{env_name} must be valid base64"));

    if decoded.len() != 32 {
        panic!(
            "{env_name} must decode to exactly 32 bytes; got {}",
            decoded.len()
        );
    }

    let mut key = [0_u8; 32];
    key.copy_from_slice(&decoded);
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
