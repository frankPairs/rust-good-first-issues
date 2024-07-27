use crate::{
    github::handlers::{get_github_repositories, get_github_repository_good_first_issues},
    state::AppState,
};
use axum::{handler::Handler, middleware, routing, Router};
use std::sync::Arc;

use super::{
    extractors::GithubGoodFirstIssuesKeyGenerator,
    models::GetGithubRepositoryGoodFirstIssuesResponse,
};
use super::{extractors::GithubRepositoriesKeyGenerator, models::GetGithubRepositoriesResponse};
use crate::redis_utils::middlewares::with_redis_cache;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route(
                "/repositories",
                routing::get(get_github_repositories).layer(middleware::from_fn_with_state(
                    state.clone(),
                    with_redis_cache::<
                        GithubRepositoriesKeyGenerator,
                        GetGithubRepositoriesResponse,
                    >,
                )),
            )
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(get_github_repository_good_first_issues.layer(
                    middleware::from_fn_with_state(
                        state.clone(),
                        with_redis_cache::<
                            GithubGoodFirstIssuesKeyGenerator,
                            GetGithubRepositoryGoodFirstIssuesResponse,
                        >,
                    ))
                ),
            )
    }
}
