pub mod handlers;
pub mod models;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_emotes))
        .route("/me", get(handlers::list_my_emotes))
        .route("/purchase", post(handlers::purchase_emote))
        .route("/upload", post(handlers::upload_emote))
}
