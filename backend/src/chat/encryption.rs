use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::rand_core::{OsRng, RngCore};
use sqlx::PgPool;
use tracing::{error, warn};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// Checked alternative to `Nonce::from_slice` that returns an error instead of
/// panicking when the slice length is not exactly `NONCE_LEN` (12 bytes).
fn checked_nonce(bytes: &[u8]) -> AppResult<[u8; NONCE_LEN]> {
    bytes.try_into().map_err(|_| {
        error!(
            expected = NONCE_LEN,
            actual = bytes.len(),
            "nonce has invalid length"
        );
        AppError::Internal("nonce has invalid length".to_owned())
    })
}

/// Checked alternative to `Aes256Gcm::new_from_slice` that validates the key
/// length before construction, returning an error instead of relying solely on
/// the downstream `InvalidLength` error.
fn checked_cipher(key: &[u8]) -> AppResult<Aes256Gcm> {
    if key.len() != KEY_LEN {
        error!(
            expected = KEY_LEN,
            actual = key.len(),
            "AES-256-GCM key has invalid length"
        );
        return Err(AppError::Internal("AES-256-GCM key has invalid length".to_owned()));
    }
    Aes256Gcm::new_from_slice(key).map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn encrypt_for_chat(
    db: &PgPool,
    chat_id: Uuid,
    content: &str,
    wrapping_key: &[u8; KEY_LEN],
    legacy_wrapping_keys: &[[u8; KEY_LEN]],
) -> AppResult<(Vec<u8>, Vec<u8>)> {
    let key = get_or_create_key(db, chat_id, wrapping_key, legacy_wrapping_keys).await?;
    let cipher = checked_cipher(&key)?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce_arr = checked_nonce(&nonce_bytes)?;

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_arr), content.as_bytes())
        .map_err(|e| AppError::Internal(format!("encryption failed: {e}")))?;

    Ok((ciphertext, nonce_arr.to_vec()))
}

pub async fn decrypt_for_chat(
    db: &PgPool,
    chat_id: Uuid,
    content_encrypted: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    legacy_content_text: Option<String>,
    wrapping_key: &[u8; KEY_LEN],
    legacy_wrapping_keys: &[[u8; KEY_LEN]],
) -> AppResult<String> {
    if let (Some(ciphertext), Some(nonce_bytes)) = (content_encrypted, nonce) {
        let nonce_arr = checked_nonce(&nonce_bytes)?;
        let key = get_or_create_key(db, chat_id, wrapping_key, legacy_wrapping_keys).await?;
        let cipher = checked_cipher(&key)?;

        let decrypted = cipher
            .decrypt(Nonce::from_slice(&nonce_arr), ciphertext.as_ref())
            .map_err(|e| AppError::Internal(format!("decryption failed: {e}")))?;

        return String::from_utf8(decrypted).map_err(|e| AppError::Internal(format!("decrypted content is not valid UTF-8: {e}")));
    }

    Ok(legacy_content_text.unwrap_or_default())
}

async fn try_unwrap_and_migrate(
    db: &PgPool,
    chat_id: Uuid,
    stored: &[u8],
    wrapping_key: &[u8; KEY_LEN],
    legacy_wrapping_keys: &[[u8; KEY_LEN]],
) -> AppResult<Vec<u8>> {
    if stored.len() == KEY_LEN {
        let wrapped = wrap_key(stored, wrapping_key)?;
        sqlx::query(
            r#"
            UPDATE chat_keys
            SET key_encrypted = $2, rotated_at = NOW()
            WHERE chat_id = $1
            "#,
        )
        .bind(chat_id)
        .bind(wrapped)
        .execute(db)
        .await?;
        return Ok(stored.to_vec());
    }

    if let Ok(unwrapped) = unwrap_key(stored, wrapping_key) {
        return Ok(unwrapped);
    }

    for legacy_key in legacy_wrapping_keys {
        if let Ok(unwrapped) = unwrap_key(stored, legacy_key) {
            let wrapped = wrap_key(unwrapped.as_slice(), wrapping_key)?;
            sqlx::query(
                r#"
                UPDATE chat_keys
                SET key_encrypted = $2, rotated_at = NOW()
                WHERE chat_id = $1
                "#,
            )
            .bind(chat_id)
            .bind(wrapped)
            .execute(db)
            .await?;
            warn!(
                chat_id = %chat_id,
                "migrated chat key from legacy wrapping key to current wrapping key"
            );
            return Ok(unwrapped);
        }
    }

    error!(
        chat_id = %chat_id,
        legacy_keys_tried = legacy_wrapping_keys.len(),
        "failed to unwrap chat key with current or any legacy wrapping key"
    );
    Err(AppError::Internal("failed to unwrap chat key with current or any legacy wrapping key".to_owned()))
}

async fn get_or_create_key(
    db: &PgPool,
    chat_id: Uuid,
    wrapping_key: &[u8; KEY_LEN],
    legacy_wrapping_keys: &[[u8; KEY_LEN]],
) -> AppResult<Vec<u8>> {
    if let Some(stored) = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT key_encrypted
        FROM chat_keys
        WHERE chat_id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(db)
    .await?
    {
        return try_unwrap_and_migrate(db, chat_id, &stored, wrapping_key, legacy_wrapping_keys)
            .await;
    }

    let mut key = vec![0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);

    let wrapped = wrap_key(&key, wrapping_key)?;
    sqlx::query(
        r#"
        INSERT INTO chat_keys (chat_id, key_encrypted)
        VALUES ($1, $2)
        ON CONFLICT (chat_id) DO NOTHING
        "#,
    )
    .bind(chat_id)
    .bind(&wrapped)
    .execute(db)
    .await?;

    let stored = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT key_encrypted
        FROM chat_keys
        WHERE chat_id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::Internal("chat key not found after insert".to_owned()))?;

    try_unwrap_and_migrate(db, chat_id, &stored, wrapping_key, legacy_wrapping_keys).await
}

fn wrap_key(raw_key: &[u8], wrapping_key: &[u8; KEY_LEN]) -> AppResult<Vec<u8>> {
    if raw_key.len() != KEY_LEN {
        return Err(AppError::Internal("raw key length mismatch in wrap_key".to_owned()));
    }

    let cipher = checked_cipher(wrapping_key)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce_arr = checked_nonce(&nonce_bytes)?;
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce_arr), raw_key)
        .map_err(|e| AppError::Internal(format!("key wrapping encryption failed: {e}")))?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend(encrypted);
    Ok(payload)
}

fn unwrap_key(payload: &[u8], wrapping_key: &[u8; KEY_LEN]) -> AppResult<Vec<u8>> {
    if payload.len() <= NONCE_LEN {
        return Err(AppError::Internal("wrapped key payload too short".to_owned()));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let nonce_arr = checked_nonce(nonce_bytes)?;
    let cipher = checked_cipher(wrapping_key)?;
    let decrypted = cipher
        .decrypt(Nonce::from_slice(&nonce_arr), ciphertext)
        .map_err(|e| AppError::Internal(format!("key unwrapping decryption failed: {e}")))?;

    if decrypted.len() != KEY_LEN {
        return Err(AppError::Internal("unwrapped key length mismatch".to_owned()));
    }

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_nonce_valid_12_bytes() {
        let bytes = [0u8; NONCE_LEN];
        let result = checked_nonce(&bytes);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), bytes);
    }

    #[test]
    fn checked_nonce_too_short() {
        let bytes = [0u8; 8];
        assert!(checked_nonce(&bytes).is_err());
    }

    #[test]
    fn checked_nonce_too_long() {
        let bytes = [0u8; 16];
        assert!(checked_nonce(&bytes).is_err());
    }

    #[test]
    fn checked_nonce_empty() {
        let bytes: [u8; 0] = [];
        assert!(checked_nonce(&bytes).is_err());
    }

    #[test]
    fn checked_cipher_valid_32_byte_key() {
        let key = [0u8; KEY_LEN];
        assert!(checked_cipher(&key).is_ok());
    }

    #[test]
    fn checked_cipher_too_short_key() {
        let key = [0u8; 16];
        assert!(checked_cipher(&key).is_err());
    }

    #[test]
    fn checked_cipher_too_long_key() {
        let key = [0u8; 64];
        assert!(checked_cipher(&key).is_err());
    }

    #[test]
    fn wrap_unwrap_roundtrip() {
        let wrapping_key = [42u8; KEY_LEN];
        let raw_key = [7u8; KEY_LEN];

        let wrapped = wrap_key(&raw_key, &wrapping_key).unwrap();
        // Wrapped payload should be nonce (12) + ciphertext (32 + 16 GCM tag)
        assert!(wrapped.len() > NONCE_LEN + KEY_LEN);

        let unwrapped = unwrap_key(&wrapped, &wrapping_key).unwrap();
        assert_eq!(unwrapped, raw_key);
    }

    #[test]
    fn unwrap_with_wrong_key_fails() {
        let wrapping_key = [42u8; KEY_LEN];
        let wrong_key = [99u8; KEY_LEN];
        let raw_key = [7u8; KEY_LEN];

        let wrapped = wrap_key(&raw_key, &wrapping_key).unwrap();
        assert!(unwrap_key(&wrapped, &wrong_key).is_err());
    }

    #[test]
    fn unwrap_too_short_payload_fails() {
        let wrapping_key = [42u8; KEY_LEN];
        let short_payload = [0u8; NONCE_LEN]; // exactly NONCE_LEN, no ciphertext
        assert!(unwrap_key(&short_payload, &wrapping_key).is_err());
    }

    #[test]
    fn wrap_key_wrong_length_raw_key_fails() {
        let wrapping_key = [42u8; KEY_LEN];
        let bad_raw_key = [7u8; 16]; // wrong length
        assert!(wrap_key(&bad_raw_key, &wrapping_key).is_err());
    }
}
