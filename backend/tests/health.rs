use oobleck_backend::routes::health;

#[tokio::test]
async fn test_health_endpoint_returns_ok() {
    let response = health().await;
    assert_eq!(response.status, "ok");
}
