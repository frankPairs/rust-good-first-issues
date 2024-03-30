use axum::extract::Path;
use axum::response::Response;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

use crate::errors::RustGoodFirstIssuesError;
use crate::state::AppState;

use crate::github_repositories::models::GetRustRepositoriesParams;
use crate::github_repositories::repositories::GithubRepository;

use super::models::{
    GetRustRepositoryGoodFirstIssuesParams, GetRustRepositoryGoodFirstIssuesPathParams,
};

#[tracing::instrument(name = "Get rust repositories")]
pub async fn get_rust_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetRustRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let repo = GithubRepository::new(state.github_settings.clone())?;

    let rust_repositories = repo.get_rust_repositories(params.0).await?;

    return Ok((StatusCode::OK, Json(rust_repositories)).into_response());
}

#[tracing::instrument(name = "Get rust repository good first issues")]
pub async fn get_rust_repository_good_first_issue(
    state: State<Arc<AppState>>,
    path_params: Path<GetRustRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetRustRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let repo = GithubRepository::new(state.github_settings.clone())?;

    let rust_repositories = repo
        .get_rust_repository_issues(path_params.0, params.0)
        .await?;

    return Ok((StatusCode::OK, Json(rust_repositories)).into_response());
}
