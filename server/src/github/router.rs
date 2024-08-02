use crate::{
    github::handlers::{get_github_repositories, get_github_repository_good_first_issues},
    redis_utils::middlewares::RedisCacheOptions,
    state::AppState,
};
use axum::{handler::Handler, routing, Router};
use std::sync::Arc;
use tower::ServiceBuilder;

use super::{
    middlewares::GithubRateLimitLayer,
    models::{GetGithubRepositoriesResponse, GetGithubRepositoryGoodFirstIssuesResponse},
};
use crate::redis_utils::middlewares::RedisCacheLayer;

const REDIS_EXPIRATION_TIME: i64 = 600;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route(
                "/repositories",
                routing::get(get_github_repositories).layer(
                    ServiceBuilder::new()
                        .layer(RedisCacheLayer::<GetGithubRepositoriesResponse>::new(
                            state.redis_pool.clone(),
                            Some(RedisCacheOptions {
                                expiration_time: Some(REDIS_EXPIRATION_TIME),
                            }),
                        ))
                        .layer(GithubRateLimitLayer::new(state.redis_pool.clone())),
                ),
            )
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(
                    get_github_repository_good_first_issues
                        .layer(ServiceBuilder::new().layer(RedisCacheLayer::<
                            GetGithubRepositoryGoodFirstIssuesResponse,
                        >::new(
                            state.redis_pool.clone(),
                            Some(RedisCacheOptions {
                                expiration_time: Some(REDIS_EXPIRATION_TIME),
                            }),
                        )))
                        .layer(GithubRateLimitLayer::new(state.redis_pool.clone())),
                ),
            )
    }
}
