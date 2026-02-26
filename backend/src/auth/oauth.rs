use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use redis::AsyncCommands;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::{
        handlers::{AuthResponse, SessionUser},
        jwt::issue_token,
    },
    error::{AppError, AppResult},
    AppState,
};

type HmacSha256 = Hmac<Sha256>;
const TELEGRAM_AUTH_TTL_SECONDS: i64 = 300;
const OAUTH_EXCHANGE_CODE_TTL_SECONDS: u64 = 60;

#[derive(Debug, Deserialize)]
pub struct OauthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub as_json: Option<bool>,
    pub id: Option<String>,
    pub auth_date: Option<String>,
    pub hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OauthExchangeRequest {
    pub code: String,
}

#[derive(Debug, FromRow)]
struct OauthUserRow {
    id: Uuid,
    username: String,
    email: Option<String>,
    is_premium: bool,
    is_age_verified: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug)]
struct OauthIdentity {
    provider_id: String,
    username: String,
    email: Option<String>,
}

pub async fn oauth_start(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    validate_provider(&provider)?;
    ensure_provider_configured(&state, provider.as_str())?;

    let state_token = CsrfToken::new_random();
    let oauth_state = state_token.secret().to_owned();

    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let _: () = conn
        .set_ex(
            oauth_state_key(oauth_state.as_str()),
            provider.as_str(),
            state.config.oauth_state_ttl_seconds,
        )
        .await?;

    let redirect_url = if provider == "telegram" {
        build_telegram_auth_url(&state, oauth_state.as_str())?
    } else {
        let client = oauth_client(&state, provider.as_str())?;
        let mut request = client.authorize_url(|| state_token);

        for scope in provider_scopes(provider.as_str()) {
            request = request.add_scope(Scope::new((*scope).to_owned()));
        }

        let (url, _) = request.url();
        url.to_string()
    };

    Ok(Json(serde_json::json!({
        "provider": provider,
        "state": oauth_state,
        "redirect_url": redirect_url
    })))
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Path((provider,)): Path<(String,)>,
    Query(query): Query<OauthCallbackQuery>,
) -> AppResult<Response> {
    validate_provider(&provider)?;
    ensure_provider_configured(&state, provider.as_str())?;
    validate_oauth_state(&state, query.state.as_deref(), provider.as_str()).await?;

    let identity = if provider == "telegram" {
        oauth_identity_from_telegram(&state, &query)?
    } else {
        oauth_identity_from_provider(&state, provider.as_str(), &query).await?
    };

    let user = find_or_create_oauth_user(&state, provider.as_str(), identity).await?;
    let token = issue_token(user.id, &state.jwt)?;

    let response = AuthResponse {
        token,
        user: SessionUser {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            is_premium: user.is_premium,
            is_age_verified: user.is_age_verified,
            created_at: user.created_at,
        },
    };

    if query.as_json.unwrap_or(false) {
        return Ok(Json(response).into_response());
    }

    let exchange_code = Uuid::new_v4().simple().to_string();
    let serialized_response = serde_json::to_string(&response).map_err(|_| AppError::Internal)?;
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let _: () = conn
        .set_ex(
            oauth_exchange_key(exchange_code.as_str()),
            serialized_response,
            OAUTH_EXCHANGE_CODE_TTL_SECONDS,
        )
        .await?;

    let redirect_url = format!(
        "{}/login?oauth_code={}",
        state.config.public_web_base_url.trim_end_matches('/'),
        url_encode(exchange_code.as_str())
    );

    Ok(Redirect::temporary(redirect_url.as_str()).into_response())
}

pub async fn oauth_exchange(
    State(state): State<AppState>,
    Json(payload): Json<OauthExchangeRequest>,
) -> AppResult<Json<AuthResponse>> {
    let code = payload.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest(
            "missing oauth exchange code".to_owned(),
        ));
    }

    if code.len() > 128 {
        return Err(AppError::BadRequest(
            "oauth exchange code is invalid".to_owned(),
        ));
    }

    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let key = oauth_exchange_key(code);
    let (serialized_response, _): (Option<String>, usize) = redis::pipe()
        .atomic()
        .get(key.as_str())
        .del(key.as_str())
        .query_async(&mut conn)
        .await?;

    let serialized_response = serialized_response.ok_or_else(|| {
        AppError::Unauthorized("oauth exchange code is invalid or expired".to_owned())
    })?;

    let response = serde_json::from_str::<AuthResponse>(serialized_response.as_str())
        .map_err(|_| AppError::Internal)?;

    Ok(Json(response))
}

async fn oauth_identity_from_provider(
    state: &AppState,
    provider: &str,
    query: &OauthCallbackQuery,
) -> AppResult<OauthIdentity> {
    let code = query
        .code
        .clone()
        .ok_or_else(|| AppError::BadRequest("missing oauth code".to_owned()))?;

    let client = oauth_client(state, provider)?;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .map_err(|_| AppError::Unauthorized("oauth code exchange failed".to_owned()))?;

    let access_token = token.access_token().secret();

    match provider {
        "google" => fetch_google_identity(access_token).await,
        "discord" => fetch_discord_identity(access_token).await,
        "github" => fetch_github_identity(access_token).await,
        _ => Err(AppError::BadRequest(
            "unsupported oauth provider".to_owned(),
        )),
    }
}

fn oauth_identity_from_telegram(
    state: &AppState,
    query: &OauthCallbackQuery,
) -> AppResult<OauthIdentity> {
    let bot_token = state
        .config
        .telegram_bot_token
        .clone()
        .ok_or_else(|| AppError::ServiceUnavailable("telegram oauth is not configured".to_owned()))?;

    let hash = query
        .hash
        .clone()
        .ok_or_else(|| AppError::BadRequest("missing telegram hash".to_owned()))?;
    let auth_date = query
        .auth_date
        .clone()
        .ok_or_else(|| AppError::BadRequest("missing telegram auth_date".to_owned()))?;
    let id = query
        .id
        .clone()
        .ok_or_else(|| AppError::BadRequest("missing telegram id".to_owned()))?;

    let auth_date_int = auth_date
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid telegram auth_date".to_owned()))?;
    let now = Utc::now().timestamp();
    if auth_date_int > now {
        return Err(AppError::Unauthorized(
            "telegram auth_date is in the future".to_owned(),
        ));
    }
    if now - auth_date_int > TELEGRAM_AUTH_TTL_SECONDS {
        return Err(AppError::Unauthorized(
            "telegram auth payload expired".to_owned(),
        ));
    }

    let mut fields = vec![("auth_date", auth_date.clone()), ("id", id.clone())];
    if let Some(first_name) = query.first_name.clone() {
        fields.push(("first_name", first_name));
    }
    if let Some(last_name) = query.last_name.clone() {
        fields.push(("last_name", last_name));
    }
    if let Some(username) = query.username.clone() {
        fields.push(("username", username));
    }

    fields.sort_by(|left, right| left.0.cmp(right.0));
    let data_check_string = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let secret = Sha256::digest(bot_token.as_bytes());
    let mut mac = HmacSha256::new_from_slice(secret.as_slice()).map_err(|_| AppError::Internal)?;
    mac.update(data_check_string.as_bytes());
    let expected = to_hex(&mac.finalize().into_bytes());

    if expected != hash.to_ascii_lowercase() {
        return Err(AppError::Unauthorized(
            "invalid telegram oauth signature".to_owned(),
        ));
    }

    let username = query
        .username
        .clone()
        .or_else(|| {
            query
                .first_name
                .as_deref()
                .map(|first_name| sanitize_username(first_name))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("telegram_{id}"));

    Ok(OauthIdentity {
        provider_id: id,
        username,
        email: None,
    })
}

async fn fetch_google_identity(access_token: &str) -> AppResult<OauthIdentity> {
    #[derive(Deserialize)]
    struct GoogleUser {
        sub: String,
        email: Option<String>,
        name: Option<String>,
    }

    let user = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| AppError::Internal)?
        .error_for_status()
        .map_err(|_| AppError::Unauthorized("google oauth userinfo failed".to_owned()))?
        .json::<GoogleUser>()
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(OauthIdentity {
        provider_id: user.sub,
        username: sanitize_username(user.name.as_deref().unwrap_or("google_user")),
        email: user.email,
    })
}

async fn fetch_discord_identity(access_token: &str) -> AppResult<OauthIdentity> {
    #[derive(Deserialize)]
    struct DiscordUser {
        id: String,
        username: String,
        discriminator: String,
        email: Option<String>,
    }

    let user = reqwest::Client::new()
        .get("https://discord.com/api/users/@me")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| AppError::Internal)?
        .error_for_status()
        .map_err(|_| AppError::Unauthorized("discord oauth userinfo failed".to_owned()))?
        .json::<DiscordUser>()
        .await
        .map_err(|_| AppError::Internal)?;

    let username = if user.discriminator == "0" {
        user.username
    } else {
        format!("{}_{}", user.username, user.discriminator)
    };

    Ok(OauthIdentity {
        provider_id: user.id,
        username: sanitize_username(username.as_str()),
        email: user.email,
    })
}

async fn fetch_github_identity(access_token: &str) -> AppResult<OauthIdentity> {
    #[derive(Deserialize)]
    struct GithubUser {
        id: u64,
        login: String,
        email: Option<String>,
    }

    #[derive(Deserialize)]
    struct GithubEmail {
        email: String,
        primary: bool,
        verified: bool,
    }

    let client = reqwest::Client::new();
    let user = client
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .header("User-Agent", "othergirl-backend")
        .send()
        .await
        .map_err(|_| AppError::Internal)?
        .error_for_status()
        .map_err(|_| AppError::Unauthorized("github oauth userinfo failed".to_owned()))?
        .json::<GithubUser>()
        .await
        .map_err(|_| AppError::Internal)?;

    let mut email = user.email;
    if email.is_none() {
        let emails = client
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("User-Agent", "othergirl-backend")
            .send()
            .await
            .map_err(|_| AppError::Internal)?
            .error_for_status()
            .map_err(|_| AppError::Unauthorized("github oauth emails failed".to_owned()))?
            .json::<Vec<GithubEmail>>()
            .await
            .map_err(|_| AppError::Internal)?;

        email = emails
            .into_iter()
            .find(|entry| entry.primary && entry.verified)
            .map(|entry| entry.email);
    }

    Ok(OauthIdentity {
        provider_id: user.id.to_string(),
        username: sanitize_username(user.login.as_str()),
        email,
    })
}

async fn find_or_create_oauth_user(
    state: &AppState,
    provider: &str,
    identity: OauthIdentity,
) -> AppResult<OauthUserRow> {
    let mut tx = state.db.begin().await?;

    let existing = sqlx::query_as::<_, OauthUserRow>(
        r#"
        SELECT u.id, u.username, u.email, u.is_premium, u.is_age_verified, u.created_at
        FROM oauth_accounts oa
        JOIN users u ON u.id = oa.user_id
        WHERE oa.provider = $1 AND oa.provider_id = $2
        "#,
    )
    .bind(provider)
    .bind(identity.provider_id.as_str())
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(user) = existing {
        tx.commit().await?;
        return Ok(user);
    }

    let mut username = identity.username;
    for _ in 0..5 {
        let created = sqlx::query_as::<_, OauthUserRow>(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, NULL)
            RETURNING id, username, email, is_premium, is_age_verified, created_at
            "#,
        )
        .bind(username.as_str())
        .bind(identity.email.as_deref())
        .fetch_one(&mut *tx)
        .await;

        let user = match created {
            Ok(user) => user,
            Err(sqlx::Error::Database(err)) if err.code().as_deref() == Some("23505") => {
                username = format!(
                    "{}_{}",
                    sanitize_username(username.as_str()),
                    &Uuid::new_v4().simple().to_string()[..6]
                );
                continue;
            }
            Err(err) => return Err(err.into()),
        };

        sqlx::query(
            r#"
            INSERT INTO oauth_accounts (user_id, provider, provider_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (provider, provider_id) DO NOTHING
            "#,
        )
        .bind(user.id)
        .bind(provider)
        .bind(identity.provider_id.as_str())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        return Ok(user);
    }

    Err(AppError::Conflict(
        "failed to reserve a unique username for oauth account".to_owned(),
    ))
}

fn oauth_client(state: &AppState, provider: &str) -> AppResult<BasicClient> {
    let (client_id, client_secret, auth_url, token_url) = match provider {
        "google" => (
            state.config.google_oauth_client_id.clone(),
            state.config.google_oauth_client_secret.clone(),
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
        ),
        "discord" => (
            state.config.discord_oauth_client_id.clone(),
            state.config.discord_oauth_client_secret.clone(),
            "https://discord.com/api/oauth2/authorize",
            "https://discord.com/api/oauth2/token",
        ),
        "github" => (
            state.config.github_oauth_client_id.clone(),
            state.config.github_oauth_client_secret.clone(),
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        ),
        _ => {
            return Err(AppError::BadRequest(
                "unsupported oauth provider".to_owned(),
            ))
        }
    };

    let client_id = client_id
        .ok_or_else(|| AppError::ServiceUnavailable(format!("{provider} oauth is not configured")))?;
    let client_secret = client_secret
        .ok_or_else(|| AppError::ServiceUnavailable(format!("{provider} oauth is not configured")))?;

    let redirect_url = format!(
        "{}/api/auth/oauth/{provider}/callback",
        state.config.public_api_base_url.trim_end_matches('/')
    );

    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url.to_owned()).map_err(|_| AppError::Internal)?,
        Some(TokenUrl::new(token_url.to_owned()).map_err(|_| AppError::Internal)?),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).map_err(|_| AppError::Internal)?);

    Ok(client)
}

fn provider_scopes(provider: &str) -> &'static [&'static str] {
    match provider {
        "google" => &["openid", "profile", "email"],
        "discord" => &["identify", "email"],
        "github" => &["read:user", "user:email"],
        _ => &[],
    }
}

fn build_telegram_auth_url(state: &AppState, oauth_state: &str) -> AppResult<String> {
    let token = state
        .config
        .telegram_bot_token
        .clone()
        .ok_or_else(|| AppError::ServiceUnavailable("telegram oauth is not configured".to_owned()))?;
    let bot_id = token
        .split(':')
        .next()
        .ok_or_else(|| AppError::ServiceUnavailable("invalid telegram bot token format".to_owned()))?;

    let callback = format!(
        "{}/api/auth/oauth/telegram/callback?state={}",
        state.config.public_api_base_url.trim_end_matches('/'),
        url_encode(oauth_state)
    );

    Ok(format!(
        "https://oauth.telegram.org/auth?bot_id={}&origin={}&return_to={}&request_access=write",
        url_encode(bot_id),
        url_encode(state.config.public_web_base_url.as_str()),
        url_encode(callback.as_str()),
    ))
}

async fn validate_oauth_state(
    state: &AppState,
    incoming_state: Option<&str>,
    provider: &str,
) -> AppResult<()> {
    let incoming_state =
        incoming_state.ok_or_else(|| AppError::BadRequest("missing oauth state".to_owned()))?;
    let mut conn = state.redis.get_multiplexed_tokio_connection().await?;
    let stored_provider: Option<String> = conn.get(oauth_state_key(incoming_state)).await?;

    if stored_provider.as_deref() != Some(provider) {
        return Err(AppError::Unauthorized("invalid oauth state".to_owned()));
    }

    let _: usize = conn.del(oauth_state_key(incoming_state)).await?;
    Ok(())
}

fn ensure_provider_configured(state: &AppState, provider: &str) -> AppResult<()> {
    let configured = match provider {
        "google" => {
            state.config.google_oauth_client_id.is_some()
                && state.config.google_oauth_client_secret.is_some()
        }
        "discord" => {
            state.config.discord_oauth_client_id.is_some()
                && state.config.discord_oauth_client_secret.is_some()
        }
        "github" => {
            state.config.github_oauth_client_id.is_some()
                && state.config.github_oauth_client_secret.is_some()
        }
        "telegram" => state.config.telegram_bot_token.is_some(),
        _ => false,
    };

    if configured {
        Ok(())
    } else {
        Err(AppError::Conflict(format!(
            "{provider} oauth provider is not configured"
        )))
    }
}

fn validate_provider(provider: &str) -> AppResult<()> {
    const ALLOWED: &[&str] = &["google", "discord", "github", "telegram"];

    if ALLOWED.contains(&provider) {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "unsupported oauth provider".to_owned(),
    ))
}

fn oauth_state_key(state: &str) -> String {
    format!("oauth_state:{state}")
}

fn oauth_exchange_key(code: &str) -> String {
    format!("oauth_exchange:{code}")
}

fn sanitize_username(value: &str) -> String {
    let normalized = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_ascii_whitespace() || ch == '-' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>();

    let fallback = if normalized.is_empty() {
        "user".to_owned()
    } else {
        normalized
    };

    fallback.chars().take(32).collect()
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => {
                let mut encoded = String::with_capacity(3);
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", byte));
                encoded.chars().collect::<Vec<_>>()
            }
        })
        .collect()
}
