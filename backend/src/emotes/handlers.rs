use std::{path::Path, str::FromStr};

use axum::{
    extract::{Multipart, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::middleware::AuthUser,
    emotes::models::Emote,
    error::{AppError, AppResult},
    payments::ledger::{self, TxType},
    AppState,
};

#[derive(Debug, serde::Deserialize)]
pub struct PurchaseEmoteRequest {
    pub emote_id: Uuid,
}

pub async fn list_emotes(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let emotes = sqlx::query_as::<_, Emote>(
        r#"
        SELECT id, token, name, image_url, price_sparks, is_active, created_by_user_id, created_at
        FROM emotes
        WHERE is_active = TRUE
        ORDER BY price_sparks ASC, created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "emotes": emotes })))
}

pub async fn list_my_emotes(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let emotes = sqlx::query_as::<_, Emote>(
        r#"
        SELECT e.id, e.token, e.name, e.image_url, e.price_sparks, e.is_active, e.created_by_user_id, e.created_at
        FROM user_emotes ue
        JOIN emotes e ON e.id = ue.emote_id
        WHERE ue.user_id = $1
        ORDER BY ue.purchased_at DESC
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "emotes": emotes })))
}

pub async fn purchase_emote(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<PurchaseEmoteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let mut tx = ledger::begin_tx(&state).await?;
    let emote = sqlx::query_as::<_, Emote>(
        r#"
        SELECT id, token, name, image_url, price_sparks, is_active, created_by_user_id, created_at
        FROM emotes
        WHERE id = $1
        "#,
    )
    .bind(payload.emote_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("emote not found".to_owned()))?;

    if !emote.is_active {
        return Err(AppError::Conflict("emote is inactive".to_owned()));
    }

    let already_owned = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM user_emotes
            WHERE user_id = $1 AND emote_id = $2
        )
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.emote_id)
    .fetch_one(&mut *tx)
    .await?;

    if already_owned {
        return Err(AppError::Conflict("emote already owned".to_owned()));
    }

    if emote.price_sparks > 0 {
        let _ = ledger::apply_spark_transaction(
            &mut tx,
            auth_user.user_id,
            -emote.price_sparks,
            TxType::FlarePurchase,
            Some(payload.emote_id),
        )
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO user_emotes (user_id, emote_id)
        VALUES ($1, $2)
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.emote_id)
    .execute(&mut *tx)
    .await?;

    ledger::commit(tx).await?;

    Ok(Json(serde_json::json!({
        "purchased": true,
        "emote_id": payload.emote_id
    })))
}

pub async fn upload_emote(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    if !state.config.admin_user_ids.contains(&auth_user.user_id) {
        return Err(AppError::Forbidden(
            "only admins can upload emotes".to_owned(),
        ));
    }

    let mut token: Option<String> = None;
    let mut name: Option<String> = None;
    let mut price_sparks = 0_i64;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext = "png".to_owned();

    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::BadRequest("invalid multipart payload".to_owned()))? {
        let field_name = field.name().unwrap_or_default().to_owned();
        match field_name.as_str() {
            "token" => {
                token = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::BadRequest("invalid token field".to_owned()))?,
                );
            }
            "name" => {
                name = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::BadRequest("invalid name field".to_owned()))?,
                );
            }
            "price_sparks" => {
                let raw = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("invalid price field".to_owned()))?;
                price_sparks = i64::from_str(raw.trim())
                    .map_err(|_| AppError::BadRequest("price_sparks must be an integer".to_owned()))?;
            }
            "file" => {
                if let Some(filename) = field.file_name() {
                    if let Some(ext) = Path::new(filename).extension().and_then(|ext| ext.to_str()) {
                        file_ext = ext.to_ascii_lowercase();
                    }
                }
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| AppError::BadRequest("invalid file field".to_owned()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let token = token.ok_or_else(|| AppError::BadRequest("missing emote token".to_owned()))?;
    let name = name.unwrap_or_else(|| token.clone());
    let file_bytes =
        file_bytes.ok_or_else(|| AppError::BadRequest("missing emote image file".to_owned()))?;

    if !is_valid_emote_token(token.as_str()) {
        return Err(AppError::BadRequest(
            "token must look like :my_emote:".to_owned(),
        ));
    }
    if price_sparks < 0 {
        return Err(AppError::BadRequest(
            "price_sparks must be >= 0".to_owned(),
        ));
    }

    tokio::fs::create_dir_all(state.config.emote_upload_dir.as_str())
        .await
        .map_err(|_| AppError::Internal)?;

    let filename = format!("{}_{}.{}", Uuid::new_v4(), auth_user.user_id, safe_extension(file_ext.as_str()));
    let filepath = format!("{}/{}", state.config.emote_upload_dir, filename);
    tokio::fs::write(filepath.as_str(), &file_bytes)
        .await
        .map_err(|_| AppError::Internal)?;

    let image_url = format!(
        "{}/{}",
        state.config.emote_public_base_url.trim_end_matches('/'),
        filename
    );

    let inserted = sqlx::query_as::<_, Emote>(
        r#"
        INSERT INTO emotes (token, name, image_url, price_sparks, is_active, created_by_user_id)
        VALUES ($1, $2, $3, $4, TRUE, $5)
        RETURNING id, token, name, image_url, price_sparks, is_active, created_by_user_id, created_at
        "#,
    )
    .bind(token.as_str())
    .bind(name.as_str())
    .bind(image_url.as_str())
    .bind(price_sparks)
    .bind(auth_user.user_id)
    .fetch_one(&state.db)
    .await;

    let emote = match inserted {
        Ok(emote) => emote,
        Err(sqlx::Error::Database(err)) if err.code().as_deref() == Some("23505") => {
            return Err(AppError::Conflict("emote token already exists".to_owned()))
        }
        Err(err) => return Err(err.into()),
    };

    Ok(Json(serde_json::json!({ "emote": emote })))
}

fn is_valid_emote_token(token: &str) -> bool {
    if token.len() < 4 || token.len() > 28 {
        return false;
    }
    if !token.starts_with(':') || !token.ends_with(':') {
        return false;
    }

    token[1..token.len() - 1]
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn safe_extension(extension: &str) -> &str {
    match extension {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => extension,
        _ => "png",
    }
}
