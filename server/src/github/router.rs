use crate::{
    github::handlers::{get_github_repositories, get_github_repository_good_first_issues},
    state::AppState,
};
use axum::{routing, Router};
use std::sync::Arc;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", routing::get(get_github_repositories))
            .route(
                "/:repo/good-first-issues",
                routing::get(get_github_repository_good_first_issues),
            )
    }
}
