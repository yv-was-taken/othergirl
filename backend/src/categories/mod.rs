pub mod handlers;
pub mod models;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_categories))
        .route("/:id", get(handlers::get_category))
        .route("/suggest", post(handlers::suggest_category))
}
