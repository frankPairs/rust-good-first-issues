use api::github::{
    client::GithubApiErrorPayload,
    models::{GetGithubRepositoryGoodFirstIssuesResponse, GithubIssueAPI, GithubIssueState},
};
use serial_test::serial;
use wiremock::{
    matchers::{method, path, query_param},
    Mock, ResponseTemplate,
};

use crate::helpers::TestApp;

const MOCK_GITHUB_REPOSITORY_ISSUES_RESPONSE: &str = r#"[
    {
        "id": 1,
        "title": "Found a bug",
        "body": "I'm having a problem with this.",
        "html_url": "https://github.com/octocat/Hello-World/issues/1347",
        "state": "open",
        "pull_request": {
            "url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347",
            "html_url": "https://github.com/octocat/Hello-World/pull/1347",
            "diff_url": "https://github.com/octocat/Hello-World/pull/1347.diff",
            "patch_url": "https://github.com/octocat/Hello-World/pull/1347.patch"
        }
    }
]"#;

#[tokio::test]
#[serial]
async fn test_get_github_repository_good_first_issues() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!(
        "{}/api/v1/github/repositories/cube/good-first-issues?owner=cube-js",
        base_url
    );
    let client = reqwest::Client::new();

    let mock_response: Vec<GithubIssueAPI> =
        serde_json::from_str(MOCK_GITHUB_REPOSITORY_ISSUES_RESPONSE).unwrap();

    Mock::given(path("/repos/cube-js/cube/issues"))
        .and(method("GET"))
        .and(query_param("labels", "good first issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .named("Get Cube repository issues from Github")
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
    let body: GetGithubRepositoryGoodFirstIssuesResponse = res.json().await.unwrap();

    assert_eq!(status, 200);
    assert!(!headers.contains_key("Cache-Control"));

    let item = body.items.first().unwrap().clone();

    assert_eq!(item.id, 1);
    assert_eq!(item.title, "Found a bug");
    assert_eq!(item.description, None);
    assert_eq!(
        item.body,
        Some("I'm having a problem with this.".to_string())
    );
    assert_eq!(
        item.url,
        "https://github.com/octocat/Hello-World/issues/1347"
    );
    assert_eq!(item.state, GithubIssueState::Open);
    assert_eq!(
        item.pull_request.unwrap().url,
        "https://github.com/octocat/Hello-World/pull/1347"
    );

    let redis_key = "api:v1:github:repositories:cube:good-first-issues:owner=cube-js".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repository_good_first_issues_from_redis() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!(
        "{}/api/v1/github/repositories/cube/good-first-issues?owner=cube-js",
        base_url
    );
    let client = reqwest::Client::new();

    let mock_response: Vec<GithubIssueAPI> =
        serde_json::from_str(MOCK_GITHUB_REPOSITORY_ISSUES_RESPONSE).unwrap();

    Mock::given(path("/repos/cube-js/cube/issues"))
        .and(method("GET"))
        .and(query_param("labels", "good first issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .named("Get Cube repository issues from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();
    let headers: axum::http::HeaderMap = res.headers().clone();
    let body: GetGithubRepositoryGoodFirstIssuesResponse = res.json().await.unwrap();

    assert_eq!(status, 200);
    assert!(headers.contains_key("Cache-Control"));

    let item = body.items.first().unwrap().clone();

    assert_eq!(item.id, 1);
    assert_eq!(item.title, "Found a bug");
    assert_eq!(item.description, None);
    assert_eq!(
        item.body,
        Some("I'm having a problem with this.".to_string())
    );
    assert_eq!(
        item.url,
        "https://github.com/octocat/Hello-World/issues/1347"
    );
    assert_eq!(item.state, GithubIssueState::Open);
    assert_eq!(
        item.pull_request.unwrap().url,
        "https://github.com/octocat/Hello-World/pull/1347"
    );

    let redis_key = "api:v1:github:repositories:cube:good-first-issues:owner=cube-js".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repository_good_first_issues_error() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!(
        "{}/api/v1/github/repositories/cube/good-first-issues?owner=cube-js",
        base_url
    );
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Validation Failed"
        }"#,
    )
    .unwrap();

    Mock::given(path("/repos/cube-js/cube/issues"))
        .and(method("GET"))
        .and(query_param("labels", "good first issue"))
        .respond_with(ResponseTemplate::new(400).set_body_json(mock_github_error))
        .named("Throw error when getting cube repository issues from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    // Second request should return the results from Redis
    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();

    assert_eq!(status, 400);

    let redis_key =
        "errors:rate_limit:api:v1:github:repositories:cube:good-first-issues".to_string();

    app.redis_json_del(redis_key).await;
}

#[tokio::test]
#[serial]
async fn test_get_github_repository_good_first_issues_rate_limit_error() {
    let app = TestApp::new().await;
    let base_url = app.spawn_app().await;

    let url = format!(
        "{}/api/v1/github/repositories/cube/good-first-issues?owner=cube-js",
        base_url
    );
    let client = reqwest::Client::new();

    let mock_github_error: GithubApiErrorPayload = serde_json::from_str(
        r#"{
            "message": "Validation Failed"
        }"#,
    )
    .unwrap();

    Mock::given(path("/repos/cube-js/cube/issues"))
        .and(method("GET"))
        .and(query_param("labels", "good first issue"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(mock_github_error)
                .append_header("retry-after", "60"),
        )
        .named("Throw rate limit error when getting cube repository issues from Github")
        .expect(1)
        .mount(&app.github_server)
        .await;

    // Second request should return the results from Redis
    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let status = res.status();

    assert_eq!(status, 429);

    let redis_key =
        "errors:rate_limit:api:v1:github:repositories:cube:good-first-issues".to_string();

    app.redis_json_del(redis_key).await;
}
