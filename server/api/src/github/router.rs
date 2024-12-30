use crate::{
    github::handlers::{get_repositories, get_repository_good_first_issues},
    state::AppState,
};
use axum::{handler::Handler, middleware, routing, Router};
use axum_redis_cache::middlewares::RedisCacheLayerBuilder;
use std::sync::Arc;

use super::{
    middlewares::rate_limit_middleware,
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
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_middleware,
            ))
            .with_state(state)
    }
}
