use api::github::client::GithubApiErrorPayload;
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serial_test::serial;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::TestApp;

#[tokio::test]
#[serial]
async fn test_different_error_than_rate_limit() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(mock_github_error)
                .append_header("retry-after", "60"),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit: bool = redis_conn.exists(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 401);
    assert!(!contains_rate_limit);

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_save_rate_limit_error_on_redis_when_retry_after_greater_than_0() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("retry-after", "60"),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit: bool = redis_conn.exists(&redis_key).await.unwrap();
    let rate_limit_expiration_time: i64 = redis_conn.ttl(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 429);
    assert!(contains_rate_limit);
    assert_eq!(rate_limit_expiration_time, 60);

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_not_save_rate_limit_error_on_redis_when_retry_after_equals_to_0() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("retry-after", "0"),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit: bool = redis_conn.exists(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 429);
    assert!(!contains_rate_limit);
}

#[tokio::test]
#[serial]
async fn test_not_save_rate_limit_error_on_redis_ratelimit_remaining_is_greater_than_0() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("x-ratelimit-remaining", "120"),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit_error: bool = redis_conn.exists(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 429);
    assert!(!contains_rate_limit_error);
}

#[tokio::test]
#[serial]
async fn test_not_save_rate_limit_error_on_redis_ratelimit_reset_is_equals_to_0() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("x-ratelimit-reset", "0"),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit_error: bool = redis_conn.exists(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 429);
    assert!(!contains_rate_limit_error);
}

#[tokio::test]
#[serial]
async fn test_save_rate_limit_error_on_redis_when_ratelimit_remaining_equals_to_0_and_ratelimit_reset_greater_than_0(
) {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Too many requests"
        }"#,
    )
    .unwrap();
    let tomorrow = Utc::now() + Duration::days(1);

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("x-ratelimit-remaining", "0")
                .append_header("x-ratelimit-reset", tomorrow.timestamp().to_string()),
        )
        .named("Throw rate limit error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();
    let mut redis_conn = app.redis_connection().await;

    let contains_rate_limit_error: bool = redis_conn.exists(&redis_key).await.unwrap();
    let rate_limit_expiration_time: i64 = redis_conn.ttl(&redis_key).await.unwrap();

    assert_eq!(res.status().as_u16(), 429);
    assert!(contains_rate_limit_error);
    // The comparison between today and tomorrow gives as a result one second less than 24 hours
    assert_eq!(rate_limit_expiration_time, 86399);

    app.redis_json_del(redis_key).await;
}
