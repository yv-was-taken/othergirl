use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand_core::{OsRng, RngCore};
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub async fn encrypt_for_chat(
    db: &PgPool,
    chat_id: Uuid,
    content: &str,
    wrapping_key: &[u8; KEY_LEN],
    legacy_wrapping_keys: &[[u8; KEY_LEN]],
) -> AppResult<(Vec<u8>, Vec<u8>)> {
    let key = get_or_create_key(db, chat_id, wrapping_key, legacy_wrapping_keys).await?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal)?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), content.as_bytes())
        .map_err(|_| AppError::Internal)?;

    Ok((ciphertext, nonce_bytes.to_vec()))
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
        let key = get_or_create_key(db, chat_id, wrapping_key, legacy_wrapping_keys).await?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal)?;

        let decrypted = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|_| AppError::Internal)?;

        return String::from_utf8(decrypted).map_err(|_| AppError::Internal);
    }

    Ok(legacy_content_text.unwrap_or_default())
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
        if stored.len() == KEY_LEN {
            let wrapped = wrap_key(&stored, wrapping_key)?;
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
            return Ok(stored);
        }

        if let Ok(unwrapped) = unwrap_key(stored.as_slice(), wrapping_key) {
            return Ok(unwrapped);
        }

        for legacy_key in legacy_wrapping_keys {
            if let Ok(unwrapped) = unwrap_key(stored.as_slice(), legacy_key) {
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

        return Err(AppError::Internal);
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
    .ok_or(AppError::Internal)?;

    if let Ok(unwrapped) = unwrap_key(stored.as_slice(), wrapping_key) {
        return Ok(unwrapped);
    }

    for legacy_key in legacy_wrapping_keys {
        if let Ok(unwrapped) = unwrap_key(stored.as_slice(), legacy_key) {
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

    Err(AppError::Internal)
}

fn wrap_key(raw_key: &[u8], wrapping_key: &[u8; KEY_LEN]) -> AppResult<Vec<u8>> {
    if raw_key.len() != KEY_LEN {
        return Err(AppError::Internal);
    }

    let cipher = Aes256Gcm::new_from_slice(wrapping_key).map_err(|_| AppError::Internal)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), raw_key)
        .map_err(|_| AppError::Internal)?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend(encrypted);
    Ok(payload)
}

fn unwrap_key(payload: &[u8], wrapping_key: &[u8; KEY_LEN]) -> AppResult<Vec<u8>> {
    if payload.len() <= NONCE_LEN {
        return Err(AppError::Internal);
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(wrapping_key).map_err(|_| AppError::Internal)?;
    let decrypted = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| AppError::Internal)?;

    if decrypted.len() != KEY_LEN {
        return Err(AppError::Internal);
    }

    Ok(decrypted)
}
