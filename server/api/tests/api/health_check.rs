use crate::helpers::TestApp;

#[tokio::test]
async fn test_health_check_returns_ok_response() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/health-check", base_url);
    let client = reqwest::Client::new();

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    assert_eq!(res.status(), 200);
}
