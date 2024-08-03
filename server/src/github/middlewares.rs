use axum::{
    extract::Request,
    response::{IntoResponse, Response},
    Extension, RequestPartsExt,
};

use futures_util::future::BoxFuture;
use reqwest::StatusCode;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::{
    layer::util::{Identity, Stack},
    Layer, Service, ServiceBuilder,
};

use super::errors::GithubRateLimitError;
use crate::{redis_utils::repositories::RedisRepository, state::AppState};

const REDIS_KEY_DELIMITER: &str = ":";

#[derive(Clone)]
pub struct GithubRateLimitLayer;

impl GithubRateLimitLayer {
    pub fn new() -> GithubRateLimitLayer {
        GithubRateLimitLayer {}
    }
}

impl<'a, S> Layer<S> for GithubRateLimitLayer {
    type Service = GithubRateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GithubRateLimitMiddleware { inner }
    }
}

pub struct GithubRateLimitServiceBuilder;

impl GithubRateLimitServiceBuilder {
    pub fn new(
        state: Arc<AppState>,
    ) -> ServiceBuilder<Stack<GithubRateLimitLayer, Stack<Extension<Arc<AppState>>, Identity>>>
    {
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
        let url = request.uri();
        let formatted_path = url.path().to_string().replace("/", REDIS_KEY_DELIMITER);
        let redis_key = format!("errors:rate_limit:{}", formatted_path).replacen(":", "", 1);

        let (mut parts, body) = request.into_parts();
        let request = Request::from_parts(parts.clone(), body);

        let future = self.inner.call(request);

        Box::pin(async move {
            let Extension(state) = match parts.extract::<Extension<Arc<AppState>>>().await {
                Ok(state) => state,
                Err(err) => {
                    return Ok(err.into_response());
                }
            };

            let mut redis_repo = match RedisRepository::new(&state.redis_pool).await {
                Ok(repo) => repo,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };
            let contains_rate_limit = match redis_repo.contains(redis_key.clone()).await {
                Ok(value) => value,
                Err(err) => {
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
                }
            };

            if contains_rate_limit {
                return Ok(
                    (StatusCode::TOO_MANY_REQUESTS, "Limit of requests exceeded").into_response(),
                );
            }

            let res: Response = future.await?;

            if res.status() != StatusCode::TOO_MANY_REQUESTS {
                return Ok(res);
            }

            let res_headers = res.headers().clone();
            let rate_limit_error = GithubRateLimitError::from_response_headers(&res_headers);

            if let Err(err) = redis_repo
                .set(
                    redis_key,
                    rate_limit_error,
                    Some(rate_limit_error.get_expiration_time()),
                )
                .await
            {
                tracing::error!("{}", err);

                return Ok((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response());
            };

            Ok((StatusCode::TOO_MANY_REQUESTS, res_headers).into_response())
        })
    }
}
