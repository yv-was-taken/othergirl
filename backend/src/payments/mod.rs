pub mod handlers;
pub mod ledger;
pub mod stripe;

use axum::{routing::get, routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/subscribe", post(handlers::subscribe))
        .route("/buy-sparks", post(handlers::buy_sparks))
        .route("/webhook", post(handlers::webhook))
        .route("/balance", get(handlers::balance))
        .route("/transactions", get(handlers::transactions))
        .route("/cashout/connect", post(handlers::cashout_connect))
        .route("/cashout/request", post(handlers::cashout_request))
        .route("/cashout/status", get(handlers::cashout_status))
}
