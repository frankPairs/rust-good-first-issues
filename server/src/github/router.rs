use crate::{
    github::handlers::{get_github_repositories, get_github_repository_good_first_issues},
    state::AppState,
};
use axum::{handler::Handler, routing, Router};
use std::sync::Arc;

use super::models::{GetGithubRepositoriesResponse, GetGithubRepositoryGoodFirstIssuesResponse};
use crate::redis_utils::middlewares::RedisCacheLayer;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route(
                "/repositories",
                routing::get(get_github_repositories).layer(RedisCacheLayer::<
                    GetGithubRepositoriesResponse,
                >::new(
                    state.redis_pool.clone()
                )),
            )
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(
                    get_github_repository_good_first_issues.layer(RedisCacheLayer::<
                        GetGithubRepositoryGoodFirstIssuesResponse,
                    >::new(
                        state.redis_pool.clone()
                    )),
                ),
            )
    }
}
