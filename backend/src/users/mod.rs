pub mod handlers;
pub mod models;
pub mod reputation;

use axum::{routing::get, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(handlers::get_me).put(handlers::update_me))
        .route(
            "/me/flare",
            get(handlers::get_my_flare).put(handlers::update_my_flare),
        )
}
