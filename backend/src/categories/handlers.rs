use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::{jwt::verify_token, middleware::AuthUser},
    categories::models::{Category, Language, SuggestCategoryRequest},
    error::{AppError, AppResult},
    AppState,
};

pub async fn list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Category>>> {
    let allow_nsfw = viewer_allows_nsfw(&state, &headers).await?;

    let categories = sqlx::query_as::<_, Category>(
        r#"
        SELECT id, name, slug, description, parent_id, is_nsfw, active_user_count,
               visibility_threshold, is_visible, sort_order, created_at
        FROM categories
        WHERE is_visible = TRUE
          AND ($1 OR is_nsfw = FALSE)
        ORDER BY sort_order ASC, name ASC
        "#,
    )
    .bind(allow_nsfw)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(categories))
}

pub async fn get_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Category>> {
    let allow_nsfw = viewer_allows_nsfw(&state, &headers).await?;

    let category = sqlx::query_as::<_, Category>(
        r#"
        SELECT id, name, slug, description, parent_id, is_nsfw, active_user_count,
               visibility_threshold, is_visible, sort_order, created_at
        FROM categories
        WHERE id = $1
          AND ($2 OR is_nsfw = FALSE)
        "#,
    )
    .bind(id)
    .bind(allow_nsfw)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("category not found".to_owned()))?;

    Ok(Json(category))
}

pub async fn suggest_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<SuggestCategoryRequest>,
) -> AppResult<Json<Category>> {
    if payload.name.trim().len() < 2 {
        return Err(AppError::BadRequest(
            "name must be at least 2 characters".to_owned(),
        ));
    }

    let slug = slugify(&payload.name);

    let insert = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (
            name, slug, description, parent_id, is_nsfw, is_admin_created,
            created_by_user_id, is_visible
        )
        VALUES ($1, $2, COALESCE($3, ''), $4, COALESCE($5, FALSE), FALSE, $6, FALSE)
        RETURNING id, name, slug, description, parent_id, is_nsfw, active_user_count,
                  visibility_threshold, is_visible, sort_order, created_at
        "#,
    )
    .bind(payload.name)
    .bind(slug)
    .bind(payload.description)
    .bind(payload.parent_id)
    .bind(payload.is_nsfw)
    .bind(auth_user.user_id)
    .fetch_one(&state.db)
    .await;

    let category = match insert {
        Ok(c) => c,
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            return Err(AppError::Conflict(
                "category slug already exists".to_owned(),
            ))
        }
        Err(err) => return Err(err.into()),
    };

    Ok(Json(category))
}

pub async fn list_languages(State(state): State<AppState>) -> AppResult<Json<Vec<Language>>> {
    let languages = sqlx::query_as::<_, Language>(
        r#"
        SELECT id, code, name, is_active
        FROM languages
        WHERE is_active = TRUE
        ORDER BY name ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(languages))
}

fn slugify(value: &str) -> String {
    slug::slugify(value.trim())
}

async fn viewer_allows_nsfw(state: &AppState, headers: &HeaderMap) -> AppResult<bool> {
    let user_id = extract_user_id_optional(headers, state)?;
    let Some(user_id) = user_id else {
        return Ok(false);
    };

    let age_verified = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT is_age_verified
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    Ok(age_verified)
}

fn extract_user_id_optional(headers: &HeaderMap, state: &AppState) -> AppResult<Option<Uuid>> {
    let Some(auth_header) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Ok(None);
    };

    let Some(token) = auth_header.strip_prefix("Bearer ") else {
        return Ok(None);
    };

    let claims = verify_token(token, &state.jwt)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;

    Ok(Some(user_id))
}
