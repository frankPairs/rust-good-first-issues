use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, Router},
    Json,
};
use axum_redis_utils::middlewares::{RedisCacheLayer, RedisCacheOptions};
use redis::JsonAsyncCommands;

use crate::helpers::{TestApp, TestHandlerResponse};

async fn test_handler() -> Response {
    (
        StatusCode::OK,
        Json(TestHandlerResponse {
            message: String::from("Test handler response"),
            status: 200,
        }),
    )
        .into_response()
}

#[tokio::test]
async fn test_save_api_result_on_redis() {
    let test_uuid = uuid::Uuid::new_v4();
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_uuid),
        get(test_handler).layer(RedisCacheLayer::<TestHandlerResponse>::new(
            test_app.redis_pool.clone(),
        )),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_uuid);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let mut redis_connection = test_app.redis_connection().await;

    let res: TestHandlerResponse = redis_connection
        .json_get(format!("api:test:{}", test_uuid), "$")
        .await
        .expect("Could not get handler response from Redis");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "Test handler response");
}

#[tokio::test]
async fn test_not_contain_cache_headers_when_response_from_handler() {
    let test_uuid = uuid::Uuid::new_v4();
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_uuid),
        get(test_handler).layer(RedisCacheLayer::<TestHandlerResponse>::new(
            test_app.redis_pool.clone(),
        )),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_uuid);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers().contains_key("Last-Modified"), false);
    assert_eq!(response.headers().contains_key("Cache-Control"), false);
}

#[tokio::test]
async fn test_return_cache_response_without_expiration_time() {
    let test_uuid = uuid::Uuid::new_v4();
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_uuid),
        get(test_handler).layer(RedisCacheLayer::<TestHandlerResponse>::new(
            test_app.redis_pool.clone(),
        )),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_uuid);

    // First request should save the response on Redis
    let _ = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute API request");

    // Second request should return the response from Redis
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers().contains_key("Last-Modified"), true);
    // When there is not expiration time, the Cache-Control header should not be present
    assert_eq!(response.headers().contains_key("Cache-Control"), false);
}

#[tokio::test]
async fn test_return_cache_response_with_expiration_time() {
    let test_uuid = uuid::Uuid::new_v4();
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_uuid),
        get(test_handler).layer(RedisCacheLayer::<TestHandlerResponse>::with_options(
            test_app.redis_pool.clone(),
            RedisCacheOptions {
                expiration_time: Some(500),
            },
        )),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_uuid);

    // First request should save the response on Redis
    let _ = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute API request");

    // Second request should return the response from Redis
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers().contains_key("Last-Modified"), true);
    assert_eq!(response.headers().contains_key("Cache-Control"), true);
    assert_eq!(
        response.headers().get("Cache-Control").unwrap(),
        "max-age=500"
    );
}
