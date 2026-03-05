use reqwest::StatusCode;

use crate::harness::spawn_app;

#[tokio::test]
async fn health_returns_ok() {
    let app = spawn_app().await;

    let resp = app.get("/health").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], true);
    assert_eq!(body["redis"], true);
}
