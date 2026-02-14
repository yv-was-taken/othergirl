pub mod handlers;

use axum::{
    routing::{delete, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reports", post(handlers::create_report))
        .route(
            "/blocks",
            post(handlers::create_block).get(handlers::list_blocks),
        )
        .route("/blocks/{user_id}", delete(handlers::delete_block))
}
