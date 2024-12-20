use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, Router},
    Json,
};
use axum_redis_cache::middlewares::RedisCacheLayerBuilder;
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
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_app.uuid),
        get(test_handler).layer(
            RedisCacheLayerBuilder::new(test_app.redis_pool.clone()).build::<TestHandlerResponse>(),
        ),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_app.uuid);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    let mut redis_connection = test_app.redis_connection().await;

    let redis_key = format!("api:test:{}", test_app.uuid);
    let res: TestHandlerResponse = redis_connection
        .json_get(&redis_key, "$")
        .await
        .expect("Could not get handler response from Redis");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "Test handler response");

    test_app.redis_json_del(redis_key).await;
}

#[tokio::test]
async fn test_not_contain_cache_headers_when_response_is_from_handler() {
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_app.uuid),
        get(test_handler).layer(
            RedisCacheLayerBuilder::new(test_app.redis_pool.clone()).build::<TestHandlerResponse>(),
        ),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_app.uuid);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to execute api request.");

    assert_eq!(response.status().as_u16(), 200);
    assert!(!response.headers().contains_key("Last-Modified"));
    assert!(!response.headers().contains_key("Cache-Control"));

    test_app
        .redis_json_del(format!("api:test:{}", test_app.uuid))
        .await;
}

#[tokio::test]
async fn test_return_cache_response_without_expiration_time() {
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_app.uuid),
        get(test_handler).layer(
            RedisCacheLayerBuilder::new(test_app.redis_pool.clone()).build::<TestHandlerResponse>(),
        ),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_app.uuid);

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
    // When there is not expiration time, the Cache-Control header should not be present
    assert!(!response.headers().contains_key("Cache-Control"));

    test_app
        .redis_json_del(format!("api:test:{}", test_app.uuid))
        .await;
}

#[tokio::test]
async fn test_return_cache_response_with_expiration_time() {
    let test_app = TestApp::new().await;
    let app = Router::new().route(
        &format!("/api/test/{}", test_app.uuid),
        get(test_handler).layer(
            RedisCacheLayerBuilder::new(test_app.redis_pool.clone())
                .with_expiration_time(500)
                .build::<TestHandlerResponse>(),
        ),
    );

    let test_app_url = test_app.spawn_app(app).await;

    let client = reqwest::Client::new();
    let url = format!("{}/api/test/{}", test_app_url, test_app.uuid);

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
    assert!(response.headers().contains_key("Cache-Control"));
    assert_eq!(
        response.headers().get("Cache-Control").unwrap(),
        "max-age=500"
    );

    test_app
        .redis_json_del(format!("api:test:{}", test_app.uuid))
        .await;
}
