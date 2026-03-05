pub mod handlers;

use axum::{routing::get, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stats", get(handlers::system_stats))
        .route("/users", get(handlers::list_users))
        .route(
            "/users/{id}/suspend",
            axum::routing::post(handlers::suspend_user),
        )
        .route(
            "/users/{id}/unsuspend",
            axum::routing::post(handlers::unsuspend_user),
        )
}
