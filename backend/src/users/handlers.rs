use std::collections::HashSet;

use axum::{extract::State, Json};
use uuid::Uuid;
use validator::Validate;

use serde::Deserialize;

use crate::{
    auth::middleware::AuthUser,
    auth::password::verify_password,
    error::{AppError, AppResult},
    users::models::{UpdateFlareRequest, UpdateProfileRequest, UserProfile},
    AppState,
};

pub async fn get_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let user = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT id, username, email, bio, is_premium, is_age_verified, keep_count, reputation_score, created_at, deleted_at, deletion_scheduled_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    let interests = load_interests(&state, auth_user.user_id).await?;

    Ok(Json(user_response(&user, interests)))
}

pub async fn update_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> AppResult<Json<serde_json::Value>> {
    payload.validate()?;

    let mut tx = state.db.begin().await?;

    let user = sqlx::query_as::<_, UserProfile>(
        r#"
        UPDATE users
        SET
            bio = COALESCE($2, bio),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, username, email, bio, is_premium, is_age_verified, keep_count, reputation_score, created_at, deleted_at, deletion_scheduled_at
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.bio)
    .fetch_one(&mut *tx)
    .await?;

    if let Some(interests) = payload.interest_category_ids {
        sqlx::query(
            r#"
            DELETE FROM user_interests
            WHERE user_id = $1
            "#,
        )
        .bind(auth_user.user_id)
        .execute(&mut *tx)
        .await?;

        if !interests.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO user_interests (user_id, category_id)
                SELECT $1, UNNEST($2::UUID[])
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(auth_user.user_id)
            .bind(&interests)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    let interests = load_interests(&state, auth_user.user_id).await?;

    Ok(Json(user_response(&user, interests)))
}

pub async fn get_my_flare(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let equipped = sqlx::query_as::<_, (Uuid, String, String, serde_json::Value)>(
        r#"
        SELECT f.id, f.name, f.item_type, f.asset_data
        FROM user_flare uf
        JOIN flare_items f ON f.id = uf.flare_item_id
        WHERE uf.user_id = $1 AND uf.is_equipped = TRUE
        ORDER BY f.item_type ASC
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.db)
    .await?;

    let owned = sqlx::query_as::<_, (Uuid, String, String, bool)>(
        r#"
        SELECT f.id, f.name, f.item_type, uf.is_equipped
        FROM user_flare uf
        JOIN flare_items f ON f.id = uf.flare_item_id
        WHERE uf.user_id = $1
        ORDER BY uf.purchased_at DESC
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "equipped": equipped.into_iter().map(|(id, name, item_type, asset_data)| {
            serde_json::json!({"id": id, "name": name, "item_type": item_type, "asset_data": asset_data})
        }).collect::<Vec<_>>(),
        "owned": owned.into_iter().map(|(id, name, item_type, is_equipped)| {
            serde_json::json!({"id": id, "name": name, "item_type": item_type, "is_equipped": is_equipped})
        }).collect::<Vec<_>>()
    })))
}

pub async fn update_my_flare(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateFlareRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let desired = payload.flare_item_ids;

    let mut tx = state.db.begin().await?;

    if !desired.is_empty() {
        let owned_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM user_flare
            WHERE user_id = $1 AND flare_item_id = ANY($2)
            FOR UPDATE
            "#,
        )
        .bind(auth_user.user_id)
        .bind(&desired)
        .fetch_one(&mut *tx)
        .await?;

        if owned_count != desired.len() as i64 {
            return Err(AppError::Forbidden(
                "one or more flare items are not owned by user".to_owned(),
            ));
        }

        // Enforce one equipped item per item_type
        let item_types = sqlx::query_scalar::<_, String>(
            r#"
            SELECT item_type
            FROM flare_items
            WHERE id = ANY($1)
            "#,
        )
        .bind(&desired)
        .fetch_all(&mut *tx)
        .await?;

        let mut seen = HashSet::new();
        for item_type in &item_types {
            if !seen.insert(item_type) {
                return Err(AppError::Conflict(
                    "only one flare item per type can be equipped".to_owned(),
                ));
            }
        }
    }

    sqlx::query(
        r#"
        UPDATE user_flare
        SET is_equipped = FALSE
        WHERE user_id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .execute(&mut *tx)
    .await?;

    if !desired.is_empty() {
        sqlx::query(
            r#"
            UPDATE user_flare
            SET is_equipped = TRUE
            WHERE user_id = $1 AND flare_item_id = ANY($2)
            "#,
        )
        .bind(auth_user.user_id)
        .bind(&desired)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "equipped": desired
    })))
}

fn user_response(user: &UserProfile, interests: Vec<Uuid>) -> serde_json::Value {
    serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "bio": user.bio,
        "is_premium": user.is_premium,
        "is_age_verified": user.is_age_verified,
        "keep_count": user.keep_count,
        "reputation_score": user.reputation_score,
        "created_at": user.created_at,
        "deleted_at": user.deleted_at,
        "deletion_scheduled_at": user.deletion_scheduled_at,
        "interest_category_ids": interests
    })
}

#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub password: String,
}

pub async fn delete_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<DeleteAccountRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT password_hash FROM users WHERE id = $1",
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    let hash = row
        .0
        .ok_or_else(|| {
            AppError::BadRequest(
                "account uses OAuth login; contact support to delete".to_owned(),
            )
        })?;

    if !verify_password(&hash, &payload.password)? {
        return Err(AppError::Unauthorized("incorrect password".to_owned()));
    }

    sqlx::query(
        "UPDATE users SET deleted_at = NOW(), deletion_scheduled_at = NOW() + INTERVAL '30 days', updated_at = NOW() WHERE id = $1",
    )
    .bind(auth_user.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "message": "account scheduled for deletion in 30 days"
    })))
}

pub async fn cancel_deletion(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NULL, deletion_scheduled_at = NULL, updated_at = NOW() WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(auth_user.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "account is not scheduled for deletion".to_owned(),
        ));
    }

    Ok(Json(serde_json::json!({
        "message": "account deletion cancelled"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_user() -> UserProfile {
        UserProfile {
            id: Uuid::new_v4(),
            username: "testuser".to_owned(),
            email: Some("test@example.com".to_owned()),
            bio: "hello world".to_owned(),
            is_premium: true,
            is_age_verified: false,
            keep_count: 5,
            reputation_score: 0.95,
            created_at: Utc::now(),
            deleted_at: None,
            deletion_scheduled_at: None,
        }
    }

    #[test]
    fn user_response_contains_all_fields() {
        let user = make_user();
        let interest1 = Uuid::new_v4();
        let interest2 = Uuid::new_v4();
        let resp = user_response(&user, vec![interest1, interest2]);

        assert_eq!(resp["id"], user.id.to_string());
        assert_eq!(resp["username"], "testuser");
        assert_eq!(resp["email"], "test@example.com");
        assert_eq!(resp["bio"], "hello world");
        assert_eq!(resp["is_premium"], true);
        assert_eq!(resp["is_age_verified"], false);
        assert_eq!(resp["keep_count"], 5);
        assert_eq!(resp["reputation_score"], 0.95);
        assert!(resp["created_at"].is_string());
        let ids = resp["interest_category_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn user_response_with_no_email() {
        let mut user = make_user();
        user.email = None;
        let resp = user_response(&user, vec![]);

        assert!(resp["email"].is_null());
    }

    #[test]
    fn user_response_with_empty_interests() {
        let user = make_user();
        let resp = user_response(&user, vec![]);

        let ids = resp["interest_category_ids"].as_array().unwrap();
        assert!(ids.is_empty());
    }
}

async fn load_interests(state: &AppState, user_id: Uuid) -> AppResult<Vec<Uuid>> {
    let interests = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT category_id
        FROM user_interests
        WHERE user_id = $1
        ORDER BY category_id
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(interests)
}
