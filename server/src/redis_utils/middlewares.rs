use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use futures_util::future::BoxFuture;
use http_body_util::BodyExt;
use reqwest::StatusCode;
use std::{
    fmt::Debug,
    marker::PhantomData,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use super::{extractors::ExtractRedisKey, repositories::RedisRepository};

#[derive(Clone)]
pub struct RedisCacheState {
    redis_pool: Pool<RedisConnectionManager>,
    options: Option<RedisCacheOptions>,
}

#[derive(Clone)]
pub struct RedisCacheLayer<ResponseType> {
    state: RedisCacheState,
    phantom_data: PhantomData<ResponseType>,
}

#[derive(Clone)]
pub struct RedisCacheOptions {
    pub expiration_time: Option<i64>,
}

impl<ResponseType> RedisCacheLayer<ResponseType>
where
    ResponseType: serde::de::DeserializeOwned
        + redis::FromRedisValue
        + serde::Serialize
        + Debug
        + Send
        + Sync,
{
    pub fn new(
        redis_pool: Pool<RedisConnectionManager>,
        options: Option<RedisCacheOptions>,
    ) -> RedisCacheLayer<ResponseType> {
        RedisCacheLayer {
            state: RedisCacheState {
                options,
                redis_pool,
            },
            phantom_data: PhantomData,
        }
    }
}

impl<S, ResponseType> Layer<S> for RedisCacheLayer<ResponseType>
where
    ResponseType: serde::de::DeserializeOwned
        + redis::FromRedisValue
        + serde::Serialize
        + Debug
        + Send
        + Sync,
{
    type Service = RedisCacheMiddleware<S, ResponseType>;

    fn layer(&self, inner: S) -> Self::Service {
        RedisCacheMiddleware {
            inner,
            state: self.state.clone(),
            phantom_data: PhantomData,
        }
    }
}

struct RedisResponseBuilder<'a, ResponseType> {
    redis_pool: &'a Pool<RedisConnectionManager>,
    redis_key: &'a str,
    options: Option<RedisCacheOptions>,
    phantom_data: PhantomData<ResponseType>,
}

impl<'a, ResponseType> RedisResponseBuilder<'a, ResponseType>
where
    ResponseType: serde::de::DeserializeOwned
        + redis::FromRedisValue
        + serde::Serialize
        + Debug
        + Send
        + Sync,
{
    // Builds the middleware response based on the data coming from Redis cache
    async fn build_from_cache(&self) -> Response {
        let mut redis_repo = match RedisRepository::new(&self.redis_pool).await {
            Ok(repo) => repo,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        let res: ResponseType = match redis_repo.get(self.redis_key).await {
            Ok(json) => json,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();

        let ttl_time = match redis_repo.get_ttl(self.redis_key).await {
            Ok(time) => time,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        headers.append(
            "Cache-Control",
            HeaderValue::from_str(&format!("max-age={}", ttl_time)).unwrap(),
        );

        return (StatusCode::OK, headers, Json(res)).into_response();
    }

    // Builds the middleware response based on the data coming from the db.
    // It saves the response within redis before sending it back through the middleware chain.
    async fn build_from_db(&self, res: Response) -> Response {
        let (parts, body) = res.into_parts();

        let bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };
        let res_json_str = match String::from_utf8(bytes.to_vec()) {
            Ok(json_str) => json_str,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };
        let res_body: ResponseType = match serde_json::from_str(&res_json_str) {
            Ok(body) => body,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        let expiration_time = match self.options.clone() {
            Some(options) => options.expiration_time,
            None => None,
        };

        let mut redis_repo = match RedisRepository::new(&self.redis_pool).await {
            Ok(repo) => repo,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        if let Err(err) = redis_repo
            .set(self.redis_key, res_body, expiration_time)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        };

        let mut res = Response::from_parts(parts, Body::from(bytes));

        if let Some(ttl_time) = expiration_time {
            res.headers_mut().append(
                "Cache-Control",
                HeaderValue::from_str(&format!("max-age={}", ttl_time)).unwrap(),
            );
        }

        res
    }
}

#[derive(Clone)]
pub struct RedisCacheMiddleware<S, ResponseType> {
    inner: S,
    state: RedisCacheState,
    phantom_data: PhantomData<ResponseType>,
}

impl<S, ResponseType> Service<Request> for RedisCacheMiddleware<S, ResponseType>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    ResponseType: serde::de::DeserializeOwned
        + redis::FromRedisValue
        + serde::Serialize
        + Debug
        + Send
        + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let state = self.state.clone();
        let (mut parts, body) = request.into_parts();
        let request = Request::from_parts(parts.clone(), body);

        let future = self.inner.call(request);

        Box::pin(async move {
            let ExtractRedisKey(redis_key) = match parts.extract::<ExtractRedisKey>().await {
                Ok(key) => key,
                Err((status_code, err_message)) => {
                    return Ok((status_code, err_message).into_response());
                }
            };

            let res_builder: RedisResponseBuilder<ResponseType> = RedisResponseBuilder {
                redis_pool: &state.redis_pool,
                redis_key: &redis_key,
                options: state.options,
                phantom_data: PhantomData,
            };

            let mut redis_repo = match RedisRepository::new(&state.redis_pool).await {
                Ok(repo) => repo,
                Err(err) => {
                    return Ok(err.into_response());
                }
            };

            let contains_resource_key = match redis_repo.contains(&redis_key).await {
                Ok(value) => value,
                Err(err) => {
                    return Ok(err.into_response());
                }
            };

            if contains_resource_key {
                return Ok(res_builder.build_from_cache().await);
            }

            let res: Response = future.await?;
            let res_status: StatusCode = res.status().clone();

            // If there is any response error, we return the respones before making any operation
            if res_status.is_client_error() || res_status.is_server_error() {
                return Ok(res);
            }

            Ok(res_builder.build_from_db(res).await)
        })
    }
}
