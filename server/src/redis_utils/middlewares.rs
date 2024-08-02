use axum::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
    Json,
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use futures_util::future::BoxFuture;
use http_body_util::BodyExt;
use itertools::{sorted, Itertools};
use reqwest::StatusCode;
use std::{
    fmt::Debug,
    marker::PhantomData,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use super::repositories::RedisRepository;

const REDIS_KEY_DELIMITER: &str = ":";

#[derive(Clone)]
pub struct RedisCacheLayer<ResponseType> {
    redis_pool: Pool<RedisConnectionManager>,
    options: Option<RedisCacheOptions>,
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
            redis_pool,
            options,
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
            redis_pool: self.redis_pool.clone(),
            options: self.options.clone(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct RedisCacheMiddleware<S, ResponseType> {
    inner: S,
    redis_pool: Pool<RedisConnectionManager>,
    options: Option<RedisCacheOptions>,
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
        let redis_pool = self.redis_pool.clone();
        let url = request.uri();
        let formatted_path = url.path().to_string().replace("/", REDIS_KEY_DELIMITER);
        let query_params = match url.query() {
            Some(query) => query,
            None => "",
        };
        let sorted_params = sorted(query_params.split("&")).join(REDIS_KEY_DELIMITER);

        let redis_key = format!("{}{}", formatted_path, sorted_params).replacen(":", "", 1);
        let redis_options = self.options.clone();

        let future = self.inner.call(request);

        Box::pin(async move {
            let mut redis_repo = match RedisRepository::new(&redis_pool).await {
                Ok(repo) => repo,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };

            let contains_resource_key = match redis_repo.contains(redis_key.clone()).await {
                Ok(value) => value,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };

            if contains_resource_key {
                let res: ResponseType = match redis_repo.get(redis_key.clone()).await {
                    Ok(json) => json,
                    Err(err) => {
                        return Ok(
                            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
                        );
                    }
                };

                return Ok((StatusCode::OK, Json(res)).into_response());
            }

            let res: Response = future.await?;

            let (parts, body) = res.into_parts();

            let bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };
            let res_json_str = match String::from_utf8(bytes.to_vec()) {
                Ok(json_str) => json_str,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };
            let res_body: ResponseType = match serde_json::from_str(&res_json_str) {
                Ok(body) => body,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };

            let expiration_time = match redis_options {
                Some(options) => options.expiration_time,
                None => None,
            };

            if let Err(err) = redis_repo.set(redis_key, res_body, expiration_time).await {
                return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
            };

            let res = Response::from_parts(parts, Body::from(bytes));

            Ok(res)
        })
    }
}
