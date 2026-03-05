use reqwest::StatusCode;
use serde_json::json;

use crate::harness::spawn_app;

#[tokio::test]
async fn register_with_valid_data() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let resp = app
        .post(
            "/api/auth/register",
            &json!({
                "username": username,
                "email": email,
                "password": "validpassword123",
            }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().is_some());
    assert!(body["user"]["id"].as_str().is_some());
    assert_eq!(body["user"]["username"], username);
}

#[tokio::test]
async fn register_duplicate_email_returns_409() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    // First registration should succeed
    let resp = app
        .post(
            "/api/auth/register",
            &json!({
                "username": username,
                "email": email,
                "password": "validpassword123",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Second registration with same email should fail
    let (username2, _) = app.unique_credentials();
    let resp = app
        .post(
            "/api/auth/register",
            &json!({
                "username": username2,
                "email": email,
                "password": "validpassword123",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn register_short_password_returns_400() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let resp = app
        .post(
            "/api/auth/register",
            &json!({
                "username": username,
                "email": email,
                "password": "short",
            }),
        )
        .await;

    // Validation errors map to BadRequest (400)
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_with_correct_credentials() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();
    let password = "validpassword123";

    // Register first
    app.register_user(&username, &email, password).await;

    // Login
    let resp = app
        .post(
            "/api/auth/login",
            &json!({
                "email": email,
                "password": password,
            }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().is_some());
    assert!(body["user"]["id"].as_str().is_some());
}

#[tokio::test]
async fn login_with_wrong_password_returns_401() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    // Register
    app.register_user(&username, &email, "correctpassword1")
        .await;

    // Login with wrong password
    let resp = app
        .post(
            "/api/auth/login",
            &json!({
                "email": email,
                "password": "wrongpassword1",
            }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_nonexistent_email_returns_401() {
    let app = spawn_app().await;

    let resp = app
        .post(
            "/api/auth/login",
            &json!({
                "email": "nonexistent@test.local",
                "password": "somepassword123",
            }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_token_returns_new_token() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let (token, _user_id) = app
        .register_user(&username, &email, "validpassword123")
        .await;

    let resp = app
        .authed_post("/api/auth/refresh", &token, &json!({}))
        .await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    let new_token = body["token"].as_str().unwrap();
    assert!(!new_token.is_empty());
    // New token should be different from the original
    assert_ne!(new_token, token);
}

#[tokio::test]
async fn logout_revokes_token() {
    let app = spawn_app().await;
    let (username, email) = app.unique_credentials();

    let (token, _user_id) = app
        .register_user(&username, &email, "validpassword123")
        .await;

    // Logout
    let resp = app
        .authed_post("/api/auth/logout", &token, &json!({}))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Subsequent authenticated request should fail with 401
    let resp = app.authed_get("/api/users/me", &token).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
