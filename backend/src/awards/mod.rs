pub mod handlers;
pub mod models;
pub mod service;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/send", post(handlers::send_award))
        .route("/", get(handlers::list_awards))
}
