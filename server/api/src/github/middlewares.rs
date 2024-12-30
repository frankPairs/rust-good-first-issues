use axum::{
    extract::{OriginalUri, Request},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use redis::{AsyncCommands, JsonAsyncCommands};
use redis_macros::FromRedisValue;
use serde::{Deserialize, Serialize};
use std::task::{Context, Poll};
use tower::{Layer, Service};

const REDIS_KEY_DELIMITER: &str = ":";

#[derive(Clone, Debug)]
pub struct GithubRateLimitLayer {
    redis_pool: Pool<RedisConnectionManager>,
}

impl GithubRateLimitLayer {
    pub fn new(redis_pool: Pool<RedisConnectionManager>) -> Self {
        GithubRateLimitLayer { redis_pool }
    }
}

impl<S> Layer<S> for GithubRateLimitLayer {
    type Service = GithubRateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GithubRateLimitMiddleware {
            inner,
            redis_pool: self.redis_pool.clone(),
        }
    }
}

#[derive(Clone)]
pub struct GithubRateLimitMiddleware<S> {
    redis_pool: Pool<RedisConnectionManager>,
    inner: S,
}

impl<S> Service<Request> for GithubRateLimitMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let redis_pool = self.redis_pool.clone();
        let (mut parts, body) = request.into_parts();
        let request = Request::from_parts(parts.clone(), body);

        let future = self.inner.call(request);

        Box::pin(async move {
            let original_uri = parts.extract::<OriginalUri>().await.unwrap();
            let formatted_path = original_uri
                .path()
                .to_string()
                .replace("/", REDIS_KEY_DELIMITER)
                .replacen(":", "", 1);
            // To build the Redis key of any rate limit error, we just use the url path, without taking into account
            // query or route params. This is because a rate limit error is
            let redis_key = format!("errors:rate_limit:{}", formatted_path);

            let mut redis_conn = match redis_pool.get().await {
                Ok(conn) => conn,
                Err(err) => {
                    tracing::error!("Error when connection to Redis pool: {}", err);

                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };

            if redis_conn.exists(&redis_key).await.unwrap_or(false) {
                return Ok(
                    (StatusCode::TOO_MANY_REQUESTS, "Limit of requests exceeded").into_response(),
                );
            }

            let res: Response = future.await?;
            let res_status = res.status();

            // Based on Github documentation, it is possible that there is a rate limit error when status codes
            // are 429 or 403. So when the status codes are different, we just return the response from the handler
            // For more information, you can check the official page https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api?apiVersion=2022-11-28#exceeding-the-rate-limit
            if res_status != StatusCode::TOO_MANY_REQUESTS && res_status != StatusCode::FORBIDDEN {
                return Ok(res);
            }

            let res_headers = res.headers().clone();
            let error = GithubRateLimitError::from_response_headers(&res_headers);

            if !error.is_rate_limit_exceeded() {
                return Ok(res);
            }

            if let Err(err) = redis_conn
                .json_set::<&str, &str, GithubRateLimitError, Option<String>>(
                    &redis_key, "$", &error,
                )
                .await
            {
                tracing::error!("Error when setting rate limit redis key: {}", err);

                return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
            }

            if let Err(err) = redis_conn
                .expire::<&str, bool>(&redis_key, error.get_expiration_time())
                .await
            {
                tracing::error!("Error when getting rate limit expiration time: {}", err);

                return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
            }

            Ok((StatusCode::TOO_MANY_REQUESTS, res_headers).into_response())
        })
    }
}

const DEFAULT_RATE_LIMIT_EXP: i64 = 600;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, FromRedisValue)]
pub struct GithubRateLimitError {
    // The time in seconds that you should wait before making the next request
    pub retry_after: Option<i64>,
    // The number of requests remaining in the current rate limit window
    pub ratelimit_remaining: Option<i64>,
    // The time at which the current rate limit window resets, in UTC epoch seconds
    pub ratelimit_reset: Option<i64>,
}

impl GithubRateLimitError {
    pub fn from_response_headers(headers: &HeaderMap) -> Self {
        let mut retry_after: Option<i64> = None;
        let mut ratelimit_remaining: Option<i64> = None;
        let mut ratelimit_reset: Option<i64> = None;

        if let Some(value) = headers.get("retry-after") {
            let parsed_value = value.to_str().unwrap_or("");

            retry_after = match String::from(parsed_value).parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-remaining") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_remaining = match String::from(parsed_value).parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-reset") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_reset = match String::from(parsed_value).parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        GithubRateLimitError {
            ratelimit_remaining,
            ratelimit_reset,
            retry_after,
        }
    }

    // Returns the rate limit expiration time in seconds. If the function returns a value greater than 0,
    // that value should be considered as a limit of time in seconds to do the next request to the Github API
    //
    // It applies the logic describe on the official Github API documentation:
    // https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api?apiVersion=2022-11-28#handle-rate-limit-errors-appropriately
    pub fn get_expiration_time(&self) -> i64 {
        // When retry_after contains a value, we should return it as expiration time. We do not need to do any conversion as Github API
        // already returns this value in seconds
        if let Some(retry_after) = self.retry_after {
            return retry_after;
        }

        let ratelimit_remaining = match self.ratelimit_remaining {
            Some(value) => value,
            None => i64::MAX,
        };

        // When ratelimit remaining is greater than 0, it means that we did not reach the rate limit amount of requests.
        if ratelimit_remaining > 0 {
            return 0;
        }

        let ratelimit_reset = self.ratelimit_reset.unwrap_or(0);

        if ratelimit_reset == 0 {
            return 0;
        }

        // We convert the rate limit reset from UTC epoch time to seconds.
        if let Some(reset_date) = DateTime::from_timestamp(ratelimit_reset, 0) {
            let today_date = Utc::now();
            let reset_expiration_date = reset_date.signed_duration_since(today_date);

            return reset_expiration_date.num_seconds();
        }

        DEFAULT_RATE_LIMIT_EXP
    }

    // We can consider that we exceede the rate limit when expiration time is bigger than 0.
    // For more information, you can visit the official site https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api?apiVersion=2022-11-28#handle-rate-limit-errors-appropriately
    pub fn is_rate_limit_exceeded(&self) -> bool {
        self.get_expiration_time() > 0
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn test_get_expiration_time_when_retry_after_is_present() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: Some(10),
            ratelimit_remaining: None,
            ratelimit_reset: None,
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 10);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_remaining_is_greater_than_zero() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: Some(10),
            ratelimit_reset: None,
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 0);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_reset_is_zero() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: None,
            ratelimit_reset: Some(0),
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 0);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_remaining_is_zero_and_ratelimit_reset_is_greater_than_zero(
    ) {
        let tomorrow = Utc::now() + Duration::days(1);

        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: Some(0),
            ratelimit_reset: Some(tomorrow.timestamp()),
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 86399);
    }
}
