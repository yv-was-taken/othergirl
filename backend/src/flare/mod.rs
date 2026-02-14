pub mod handlers;
pub mod models;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/items", get(handlers::list_items))
        .route("/purchase", post(handlers::purchase_item))
}
