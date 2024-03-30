use crate::{
    github_repositories::handlers::{get_rust_repositories, get_rust_repository_good_first_issue},
    state::AppState,
};
use axum::{routing, Router};
use std::sync::Arc;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build() -> Router<Arc<AppState>> {
        Router::new()
            .route("/", routing::get(get_rust_repositories))
            .route(
                "/:repo/good-first-issues",
                routing::get(get_rust_repository_good_first_issue),
            )
    }
}
