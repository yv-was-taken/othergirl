use reqwest::StatusCode;
use serde_json::json;

use crate::harness::spawn_app;

#[tokio::test]
async fn get_me_with_valid_token() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let (token, user_id) = app.register_user(&username, &email, "validpassword123").await;

    let resp = app.authed_get("/api/users/me", &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], user_id.to_string());
    assert_eq!(body["username"], username);
    assert_eq!(body["email"], email);
}

#[tokio::test]
async fn update_me_bio() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let (token, _user_id) = app.register_user(&username, &email, "validpassword123").await;

    let resp = app
        .authed_put(
            "/api/users/me",
            &token,
            &json!({
                "bio": "Hello, world!",
            }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["bio"], "Hello, world!");

    // Verify the bio persisted
    let resp = app.authed_get("/api/users/me", &token).await;
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["bio"], "Hello, world!");
}

#[tokio::test]
async fn get_me_without_token_returns_401() {
    let app = spawn_app().await;

    let resp = app.get("/api/users/me").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
