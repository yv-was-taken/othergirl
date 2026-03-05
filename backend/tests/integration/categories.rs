use reqwest::StatusCode;

use crate::harness::spawn_app;

#[tokio::test]
async fn list_categories_returns_200() {
    let app = spawn_app().await;

    let resp = app.get("/api/categories").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array(), "categories response should be an array");
}

#[tokio::test]
async fn list_languages_returns_200() {
    let app = spawn_app().await;

    let resp = app.get("/api/languages").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array(), "languages response should be an array");
}
