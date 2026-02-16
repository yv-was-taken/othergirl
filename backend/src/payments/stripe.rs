use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde::Deserialize;
use sha2::Sha256;

use crate::error::{AppError, AppResult};

const STRIPE_BASE: &str = "https://api.stripe.com/v1";
const WEBHOOK_TOLERANCE_SECONDS: i64 = 5 * 60;
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: Option<String>,
    pub mode: Option<String>,
    pub customer: Option<String>,
    pub subscription: Option<String>,
    pub client_reference_id: Option<String>,
    pub payment_status: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub customer: Option<String>,
    pub status: String,
    pub current_period_start: i64,
    pub current_period_end: i64,
}

#[derive(Debug, Deserialize)]
pub struct StripeAccount {
    pub id: String,
    pub payouts_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct StripeAccountLink {
    pub url: String,
}

pub async fn create_checkout_session(
    secret_key: &str,
    form_params: &[(String, String)],
) -> AppResult<CheckoutSession> {
    post_form::<CheckoutSession>(secret_key, "/checkout/sessions", form_params).await
}

pub async fn create_connect_account(secret_key: &str) -> AppResult<StripeAccount> {
    let params = [("type".to_owned(), "express".to_owned())];
    post_form::<StripeAccount>(secret_key, "/accounts", &params).await
}

pub async fn create_account_link(
    secret_key: &str,
    account: &str,
    refresh_url: &str,
    return_url: &str,
) -> AppResult<StripeAccountLink> {
    let params = [
        ("account".to_owned(), account.to_owned()),
        ("refresh_url".to_owned(), refresh_url.to_owned()),
        ("return_url".to_owned(), return_url.to_owned()),
        ("type".to_owned(), "account_onboarding".to_owned()),
    ];

    post_form::<StripeAccountLink>(secret_key, "/account_links", &params).await
}

pub async fn get_account(secret_key: &str, account_id: &str) -> AppResult<StripeAccount> {
    get_json::<StripeAccount>(secret_key, &format!("/accounts/{account_id}")).await
}

pub async fn get_subscription(
    secret_key: &str,
    subscription_id: &str,
) -> AppResult<StripeSubscription> {
    get_json::<StripeSubscription>(secret_key, &format!("/subscriptions/{subscription_id}")).await
}

pub async fn create_transfer(
    secret_key: &str,
    destination_account: &str,
    amount: i64,
    currency: &str,
    description: &str,
    idempotency_key: &str,
) -> AppResult<String> {
    #[derive(Debug, Deserialize)]
    struct Transfer {
        id: String,
    }

    let params = [
        ("destination".to_owned(), destination_account.to_owned()),
        ("amount".to_owned(), amount.to_string()),
        ("currency".to_owned(), currency.to_owned()),
        ("description".to_owned(), description.to_owned()),
    ];

    let transfer = post_form_with_idempotency::<Transfer>(
        secret_key,
        "/transfers",
        &params,
        Some(idempotency_key),
    )
    .await?;
    Ok(transfer.id)
}

pub fn verify_webhook_signature(
    payload: &[u8],
    signature_header: &str,
    webhook_secret: &str,
) -> AppResult<()> {
    let mut timestamp: Option<i64> = None;
    let mut signatures = Vec::new();

    for part in signature_header.split(',') {
        let mut bits = part.splitn(2, '=');
        let key = bits.next().unwrap_or_default().trim();
        let value = bits.next().unwrap_or_default().trim();

        if key == "t" {
            timestamp = value.parse::<i64>().ok();
        } else if key == "v1" {
            signatures.push(value.to_owned());
        }
    }

    let timestamp = timestamp
        .ok_or_else(|| AppError::Unauthorized("missing webhook signature timestamp".to_owned()))?;

    let now = Utc::now().timestamp();
    if (now - timestamp).abs() > WEBHOOK_TOLERANCE_SECONDS {
        return Err(AppError::Unauthorized(
            "webhook signature timestamp outside tolerance".to_owned(),
        ));
    }

    let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .map_err(|_| AppError::Unauthorized("invalid webhook signing configuration".to_owned()))?;
    mac.update(signed_payload.as_bytes());

    for signature in signatures {
        let decoded = decode_hex(signature.as_str())
            .map_err(|_| AppError::Unauthorized("invalid webhook signature encoding".to_owned()))?;
        if mac.clone().verify_slice(decoded.as_slice()).is_ok() {
            return Ok(());
        }
    }

    Err(AppError::Unauthorized(
        "webhook signature verification failed".to_owned(),
    ))
}

pub fn timestamp_to_utc(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap_or_else(Utc::now)
}

async fn get_json<T>(secret_key: &str, path: &str) -> AppResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{STRIPE_BASE}{path}"))
        .bearer_auth(secret_key)
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    parse_response::<T>(response).await
}

async fn post_form<T>(secret_key: &str, path: &str, params: &[(String, String)]) -> AppResult<T>
where
    T: serde::de::DeserializeOwned,
{
    post_form_with_idempotency(secret_key, path, params, None).await
}

async fn post_form_with_idempotency<T>(
    secret_key: &str,
    path: &str,
    params: &[(String, String)],
    idempotency_key: Option<&str>,
) -> AppResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();
    let mut request = client
        .post(format!("{STRIPE_BASE}{path}"))
        .bearer_auth(secret_key)
        .form(params);

    if let Some(idempotency_key) = idempotency_key {
        request = request.header("Idempotency-Key", idempotency_key);
    }

    let response = request.send().await.map_err(|_| AppError::Internal)?;

    parse_response::<T>(response).await
}

async fn parse_response<T>(response: reqwest::Response) -> AppResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    let body = response.text().await.map_err(|_| AppError::Internal)?;

    if !status.is_success() {
        return Err(map_stripe_error(status, body.as_str()));
    }

    serde_json::from_str::<T>(body.as_str()).map_err(|_| AppError::Internal)
}

fn map_stripe_error(status: StatusCode, body: &str) -> AppError {
    let parsed_error = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("error").cloned());

    let message = parsed_error
        .as_ref()
        .and_then(|error| {
            error
                .get("message")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "stripe request failed".to_owned());

    let error_type = parsed_error
        .as_ref()
        .and_then(|error| error.get("type").and_then(|v| v.as_str()));
    let error_code = parsed_error
        .as_ref()
        .and_then(|error| error.get("code").and_then(|v| v.as_str()));

    if status == StatusCode::TOO_MANY_REQUESTS || matches!(error_type, Some("rate_limit_error")) {
        return AppError::TooManyRequests(message);
    }

    if matches!(error_type, Some("idempotency_error")) {
        return AppError::BadRequest(message);
    }

    if matches!(error_code, Some("idempotency_key_in_use")) {
        return AppError::Conflict(message);
    }

    if status == StatusCode::UNAUTHORIZED {
        return AppError::Unauthorized(message);
    }

    if status == StatusCode::FORBIDDEN {
        return AppError::Forbidden(message);
    }

    if status.is_client_error() {
        return AppError::BadRequest(message);
    }

    AppError::Internal
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ()> {
    if value.len() % 2 != 0 {
        return Err(());
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    let chars = value.as_bytes();
    let mut idx = 0;
    while idx < chars.len() {
        let high = hex_nibble(chars[idx])?;
        let low = hex_nibble(chars[idx + 1])?;
        bytes.push((high << 4) | low);
        idx += 2;
    }

    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}
