use crate::{
    github::handlers::{get_repository_good_first_issues, get_rust_repositories},
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
                routing::get(get_repository_good_first_issues),
            )
    }
}
