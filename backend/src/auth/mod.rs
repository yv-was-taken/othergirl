pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod oauth;
pub mod password;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
        .route("/refresh", post(handlers::refresh))
        .route("/oauth/exchange", post(oauth::oauth_exchange))
        .route("/change-password", post(handlers::change_password))
        .route("/oauth/{provider}", get(oauth::oauth_start))
        .route("/oauth/{provider}/callback", get(oauth::oauth_callback))
}
