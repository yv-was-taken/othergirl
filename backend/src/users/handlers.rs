use axum::{extract::State, Json};
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::middleware::AuthUser,
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
        SELECT id, username, email, bio, is_premium, is_age_verified, keep_count, reputation_score, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    let interests = load_interests(&state, auth_user.user_id).await?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "bio": user.bio,
        "is_premium": user.is_premium,
        "is_age_verified": user.is_age_verified,
        "keep_count": user.keep_count,
        "reputation_score": user.reputation_score,
        "created_at": user.created_at,
        "interest_category_ids": interests
    })))
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
            is_age_verified = COALESCE($3, is_age_verified),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, username, email, bio, is_premium, is_age_verified, keep_count, reputation_score, created_at
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.bio)
    .bind(payload.is_age_verified)
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

    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "bio": user.bio,
        "is_premium": user.is_premium,
        "is_age_verified": user.is_age_verified,
        "keep_count": user.keep_count,
        "reputation_score": user.reputation_score,
        "created_at": user.created_at,
        "interest_category_ids": interests
    })))
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

    if !desired.is_empty() {
        let owned_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM user_flare
            WHERE user_id = $1 AND flare_item_id = ANY($2)
            "#,
        )
        .bind(auth_user.user_id)
        .bind(&desired)
        .fetch_one(&state.db)
        .await?;

        if owned_count != desired.len() as i64 {
            return Err(AppError::Forbidden(
                "one or more flare items are not owned by user".to_owned(),
            ));
        }
    }

    let mut tx = state.db.begin().await?;

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
