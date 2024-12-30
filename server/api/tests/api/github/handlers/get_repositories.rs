use api::github::{
    client::GithubApiErrorPayload,
    models::{GetGithubRepositoriesResponse, SearchGithubRepositoriesResponseAPI},
};
use serial_test::serial;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::TestApp;

const MOCK_GITHUB_REPOSITORIES_RESPONSE: &str = r#"{
    "total_count": 1,
    "items": [
        {
            "id": 1296269,
            "full_name": "octocat/Hello-World",
            "private": false,
            "html_url": "https://github.com/octocat",
            "description": "This your first repo!",
            "stargazers_count": 80,
            "open_issues_count": 0,
            "has_issues": true,
            "owner": {
                "avatar_url": "https://github.com/images/error/octocat_happy.gif"
            },
            "license": null
        }
    ]
}"#;

#[tokio::test]
#[serial]
async fn test_get_github_repositories() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_response: SearchGithubRepositoriesResponseAPI =
        serde_json::from_str(MOCK_GITHUB_REPOSITORIES_RESPONSE).unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_github_response))
        .named("Get repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();
    let headers: axum::http::HeaderMap = res.headers().clone();
    let body: GetGithubRepositoriesResponse = res.json().await.unwrap();

    assert_eq!(status, 200);
    assert!(!headers.contains_key("Cache-Control"));
    assert_eq!(body.total_count, 1);

    let item = &body.items[0];

    assert_eq!(item.id, 1296269);
    assert_eq!(item.url, "https://github.com/octocat");
    assert_eq!(item.name, "octocat/Hello-World");
    assert!(!item.private);
    assert_eq!(
        item.avatar_url,
        "https://github.com/images/error/octocat_happy.gif"
    );
    assert_eq!(item.description, Some("This your first repo!".to_string()));
    assert_eq!(item.stars_count, 80);
    assert_eq!(item.open_issues_count, 0);
    assert!(item.has_issues);
    assert_eq!(item.license, None);

    let redis_key = "api:v1:github:repositories".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repositories_from_redis() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_response: SearchGithubRepositoriesResponseAPI =
        serde_json::from_str(MOCK_GITHUB_REPOSITORIES_RESPONSE).unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_github_response))
        .named("Get repositories from Redis")
        .expect(1)
        .mount(&app.github_server)
        .await;

    // First request savez the results on Redis
    client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    // Second request should return the results from Redis
    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();
    let headers: axum::http::HeaderMap = res.headers().clone();
    let body: GetGithubRepositoriesResponse = res.json().await.unwrap();

    assert_eq!(status, 200);
    assert!(headers.contains_key("Cache-Control"));
    assert_eq!(body.total_count, 1);

    let item = &body.items[0];

    assert_eq!(item.id, 1296269);
    assert_eq!(item.url, "https://github.com/octocat");
    assert_eq!(item.name, "octocat/Hello-World");
    assert!(!item.private);
    assert_eq!(
        item.avatar_url,
        "https://github.com/images/error/octocat_happy.gif"
    );
    assert_eq!(item.description, Some("This your first repo!".to_string()));
    assert_eq!(item.stars_count, 80);
    assert_eq!(item.open_issues_count, 0);
    assert!(item.has_issues);
    assert_eq!(item.license, None);

    let redis_key = "api:v1:github:repositories".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repositories_error() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!("{}/api/v1/github/repositories", base_url);
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Validation Failed"
        }"#,
    )
    .unwrap();

    Mock::given(path("/search/repositories"))
        .and(method("GET"))
        .respond_with(ResponseTemplate::new(400).set_body_json(mock_github_error))
        .named("Throw error when getting repositories from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();

    assert_eq!(status, 400);

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repositories_rate_limit_error() {
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

    let status = res.status();

    assert_eq!(status, 429);

    let redis_key = "errors:rate_limit:api:v1:github:repositories".to_string();

    app.redis_json_del(redis_key).await;
}
