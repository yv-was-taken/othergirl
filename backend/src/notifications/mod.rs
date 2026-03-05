pub mod handlers;
pub mod models;

use axum::{routing::{get, post}, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_notifications))
        .route("/{id}/read", post(handlers::mark_read))
        .route("/read-all", post(handlers::mark_all_read))
        .route("/unread-count", get(handlers::unread_count))
}
