use axum::{
    extract::{OriginalUri, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, RequestPartsExt,
};
use futures_util::future::BoxFuture;
use redis::{AsyncCommands, JsonAsyncCommands};
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::{
    layer::util::{Identity, Stack},
    Layer, Service, ServiceBuilder,
};

use super::errors::GithubRateLimitError;
use crate::state::AppState;

const REDIS_KEY_DELIMITER: &str = ":";

#[derive(Clone)]
pub struct GithubRateLimitLayer;

impl GithubRateLimitLayer {
    pub fn new() -> GithubRateLimitLayer {
        GithubRateLimitLayer {}
    }
}

impl<S> Layer<S> for GithubRateLimitLayer {
    type Service = GithubRateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GithubRateLimitMiddleware { inner }
    }
}

pub struct GithubRateLimitServiceBuilder;

type GithubRateLimitServiceBuilderType =
    ServiceBuilder<Stack<GithubRateLimitLayer, Stack<Extension<Arc<AppState>>, Identity>>>;

impl GithubRateLimitServiceBuilder {
    pub fn build(state: Arc<AppState>) -> GithubRateLimitServiceBuilderType {
        ServiceBuilder::new()
            .layer(Extension(state))
            .layer(GithubRateLimitLayer::new())
    }
}

#[derive(Clone)]
pub struct GithubRateLimitMiddleware<S> {
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

            let redis_key = format!("errors:rate_limit:{}", formatted_path);

            let Extension(state) = match parts.extract::<Extension<Arc<AppState>>>().await {
                Ok(state) => state,
                Err(err) => {
                    tracing::error!("Error when extracting state: {}", err);

                    return Ok(err.into_response());
                }
            };
            let mut redis_conn = match state.redis_pool.get().await {
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
