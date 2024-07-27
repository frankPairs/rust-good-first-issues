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

use crate::github::models::GetGithubRepositoriesParams;

use super::models::{
    GetGithubRepositoryGoodFirstIssuesParams, GetGithubRepositoryGoodFirstIssuesPathParams,
};
use super::repositories::http::repositories::{
    GithubGoodFirstIssuesHttpRepository, GithubRepositoriesHttpRepository,
};

#[tracing::instrument(name = "Get Github repositories handler", skip(state))]
pub async fn get_github_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetGithubRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let params = params.0;

    let http_repo = GithubRepositoriesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&params).await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}

#[tracing::instrument(name = "Get Github repository good first issues", skip(state))]
pub async fn get_github_repository_good_first_issues(
    state: State<Arc<AppState>>,
    path: Path<GetGithubRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetGithubRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let params = params.0;
    let path_params = path.0;

    let http_repo = GithubGoodFirstIssuesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&path_params, &params).await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}
