use crate::{
    github::handlers::{get_github_repositories, get_github_repository_good_first_issues},
    state::AppState,
};
use axum::{handler::Handler, routing, Router};
use axum_redis_utils::middlewares::{RedisCacheLayer, RedisCacheOptions};
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
                routing::get(get_github_repositories).layer(RedisCacheLayer::<
                    GetGithubRepositoriesResponse,
                >::with_options(
                    state.redis_pool.clone(),
                    RedisCacheOptions {
                        expiration_time: Some(GITHUB_REDIS_EXPIRATION_TIME),
                    },
                )),
            )
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(
                    get_github_repository_good_first_issues.layer(RedisCacheLayer::<
                        GetGithubRepositoryGoodFirstIssuesResponse,
                    >::with_options(
                        state.redis_pool.clone(),
                        RedisCacheOptions {
                            expiration_time: Some(GITHUB_REDIS_EXPIRATION_TIME),
                        },
                    )),
                ),
            )
            .route_layer(GithubRateLimitServiceBuilder::new(state))
    }
}
