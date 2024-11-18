use crate::{
    github::handlers::{get_repositories, get_repository_good_first_issues},
    state::AppState,
};
use axum::{routing, Router};

use std::sync::Arc;

use super::middlewares::GithubRateLimitServiceBuilder;

pub struct GithubRepositoryRouter;

impl GithubRepositoryRouter {
    pub fn build(state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/repositories", routing::get(get_repositories))
            .route(
                "/repositories/:repo/good-first-issues",
                routing::get(get_repository_good_first_issues),
            )
            .route_layer(GithubRateLimitServiceBuilder::build(state))
    }
}
