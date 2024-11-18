use axum::{
    body::Body,
    extract::{OriginalUri, Request},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, RequestPartsExt,
};
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use futures_util::future::BoxFuture;
use http_body_util::BodyExt;
use redis::{AsyncCommands, FromRedisValue, JsonAsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::state::AppState;

const REDIS_KEY_DELIMITER: &str = ":";

#[derive(Clone)]
pub struct RedisCacheConfig {
    options: Option<RedisCacheOptions>,
}

#[derive(Clone)]
pub struct RedisCacheOptions {
    pub expiration_time: Option<i64>,
}

#[derive(Clone)]
pub struct RedisCacheLayer<ResponseType> {
    config: RedisCacheConfig,
    phantom_data: PhantomData<ResponseType>,
}

impl<ResponseType> RedisCacheLayer<ResponseType>
where
    ResponseType: DeserializeOwned + FromRedisValue + Serialize + Debug + Send + Sync,
{
    pub fn new() -> RedisCacheLayer<ResponseType> {
        RedisCacheLayer {
            config: RedisCacheConfig { options: None },
            phantom_data: PhantomData,
        }
    }

    pub fn with_options(options: RedisCacheOptions) -> RedisCacheLayer<ResponseType> {
        RedisCacheLayer {
            config: RedisCacheConfig {
                options: Some(options),
            },
            phantom_data: PhantomData,
        }
    }
}

impl<S, ResponseType> Layer<S> for RedisCacheLayer<ResponseType>
where
    ResponseType: DeserializeOwned + FromRedisValue + Serialize + Debug + Send + Sync,
{
    type Service = RedisCacheMiddleware<S, ResponseType>;

    fn layer(&self, inner: S) -> Self::Service {
        RedisCacheMiddleware {
            inner,
            config: self.config.clone(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct RedisCacheMiddleware<S, ResponseType> {
    inner: S,
    config: RedisCacheConfig,
    phantom_data: PhantomData<ResponseType>,
}

impl<S, ResponseType> Service<Request> for RedisCacheMiddleware<S, ResponseType>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    ResponseType: DeserializeOwned + FromRedisValue + Serialize + Debug + Send + Sync,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let config = self.config.clone();

        let (mut parts, body) = req.into_parts();

        let request = Request::from_parts(parts.clone(), body);

        let future = self.inner.call(request);

        Box::pin(async move {
            let original_uri = parts.extract::<OriginalUri>().await.unwrap();
            let redis_key = original_uri
                .path()
                .to_string()
                .replace("/", REDIS_KEY_DELIMITER)
                .replacen(":", "", 1);

            let Extension(state) = match parts.extract::<Extension<Arc<AppState>>>().await {
                Ok(state) => state,
                Err(err) => {
                    tracing::error!("Error when extracting state: {}", err);

                    return Ok(err.into_response());
                }
            };

            // It creates a new instance of the RedisResponseBuilder, which is responsible for building the response from the Redis cache or the handler.
            let mut res_builder: RedisResponseBuilder<ResponseType> =
                match RedisResponseBuilder::new(
                    &state.redis_pool,
                    &redis_key,
                    config.options.clone(),
                )
                .await
                {
                    Ok(builder) => builder,
                    Err(_) => {
                        // if there is any error while trying to connect to Redis, we return the response from the handler before making any operation
                        let res: Response = future.await?;

                        return Ok(res);
                    }
                };

            if res_builder.should_build_from_cache().await {
                return Ok(res_builder.build_from_cache().await);
            }

            let res: Response = future.await?;
            let res_status: StatusCode = res.status();

            // If there is any response error, we return the as we do not need to build the response from the Redis response builder.
            if res_status.is_client_error() || res_status.is_server_error() {
                return Ok(res);
            }

            // It builds the response from the handler and saves it to Redis before returning it.
            Ok(res_builder.build_from_handler(res).await)
        })
    }
}

// It contains the logic to build the response from the Redis cache or the handler.
struct RedisResponseBuilder<'a, ResponseType> {
    redis_conn: PooledConnection<'a, RedisConnectionManager>,
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
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
        redis_key: &'a str,
        options: Option<RedisCacheOptions>,
    ) -> Result<Self, RedisUtilsError> {
        let redis_conn = redis_pool
            .get()
            .await
            .map_err(RedisUtilsError::RedisConnectionError)?;

        Ok(Self {
            redis_conn,
            redis_key,
            options,
            phantom_data: PhantomData,
        })
    }

    // Builds the middleware response based on the data coming from Redis cache
    async fn build_from_cache(&mut self) -> Response {
        let res: ResponseType = match self.redis_conn.json_get(self.redis_key, "$").await {
            Ok(json) => json,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
            }
        };

        let mut headers: HeaderMap<HeaderValue> = HeaderMap::new();

        let expiration_time = self.redis_conn.ttl(self.redis_key).await.unwrap_or(None);

        self.set_cache_headers(&mut headers, expiration_time);

        (StatusCode::OK, headers, Json(res)).into_response()
    }

    // Builds the middleware response based on the data coming from a handler.
    // It saves the response within redis before sending it back through the middleware chain.
    async fn build_from_handler(&mut self, res: Response) -> Response {
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

        if let Err(err) = self
            .save_response_to_redis(self.redis_key, res_body, expiration_time)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        };

        Response::from_parts(parts, Body::from(bytes))
    }

    // Checks if the response should be built from the cache. If the key exists in Redis, it returns true.
    async fn should_build_from_cache(&mut self) -> bool {
        self.redis_conn
            .exists(self.redis_key)
            .await
            .unwrap_or(false)
    }

    // Saves the response from the handler to Redis.
    async fn save_response_to_redis(
        &mut self,
        key: &str,
        value: ResponseType,
        expiration_time: Option<i64>,
    ) -> Result<(), RedisUtilsError> {
        self.redis_conn
            .json_set::<&str, &str, ResponseType, ()>(key, "$", &value)
            .await
            .map_err(RedisUtilsError::RedisError)?;

        if let Some(expiration_time) = expiration_time {
            self.redis_conn
                .expire::<&str, ()>(key, expiration_time)
                .await
                .map_err(RedisUtilsError::RedisError)?;
        }

        Ok(())
    }

    // Sets the Cache-Control header using the expiration time in seconds.
    fn set_cache_headers(
        &mut self,
        headers: &mut HeaderMap<HeaderValue>,
        expiration_time: Option<i64>,
    ) {
        // If the expiration time is less than or equal to zero, it means that the key exists but it does not contain
        // any expiration time. In this case, we do not set the Cache-Control header.
        let expiration_time: Option<i64> = expiration_time.filter(|time| *time > 0);

        if let Some(expiration_time) = expiration_time {
            headers.append(
                "Cache-Control",
                HeaderValue::from_str(&format!("max-age={}", expiration_time)).unwrap(),
            );
        }
    }
}
