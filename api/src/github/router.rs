use crate::{
    github::handlers::{get_repositories, get_repository_good_first_issues},
    state::AppState,
};
use axum::{handler::Handler, routing, Router};
use axum_redis_cache::middlewares::RedisCacheLayerBuilder;
use std::sync::Arc;

use super::{
    middlewares::GithubRateLimitServiceBuilder,
    models::{GetGithubRepositoriesResponse, GetGithubRepositoryGoodFirstIssuesResponse},
};

const GITHUB_REDIS_EXPIRATION_TIME: i64 = 600;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route(
                "/repositories",
                routing::get(get_repositories).layer(
                    RedisCacheLayerBuilder::new(state.redis_pool.clone())
                        .with_expiration_time(GITHUB_REDIS_EXPIRATION_TIME)
                        .build::<GetGithubRepositoriesResponse>(),
                ),
            )
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(
                    get_repository_good_first_issues.layer(
                        RedisCacheLayerBuilder::new(state.redis_pool.clone())
                            .with_expiration_time(GITHUB_REDIS_EXPIRATION_TIME)
                            .build::<GetGithubRepositoryGoodFirstIssuesResponse>(),
                    ),
                ),
            )
            .route_layer(GithubRateLimitServiceBuilder::build(state))
    }
}
