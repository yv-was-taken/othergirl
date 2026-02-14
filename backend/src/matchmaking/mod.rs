pub mod handlers;
pub mod matcher;
pub mod queue;

use axum::{routing::delete, routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/join", post(handlers::join_queue))
        .route("/leave", delete(handlers::leave_queue))
        .route("/status", get(handlers::queue_status))
}
