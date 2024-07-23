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

use crate::github::models::GetRustRepositoriesParams;

use super::models::{
    GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
};
use super::repositories::http::repositories::{
    GoodFirstIssuesHttpRepository, RepositoriesHttpRepository,
};
use super::repositories::redis::repositories::{
    GithubGoodFirstIssuesKeyGenerator, GithubRedisRepository, GithubRepositoriesKeyGenerator,
};

#[tracing::instrument(name = "Get rust repositories", skip(state))]
pub async fn get_rust_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetRustRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = GithubRedisRepository::new(&state.redis_pool).await?;
    let params = params.0;
    let github_repository_key = GithubRepositoriesKeyGenerator { params: &params };

    if redis_repo.contains(&github_repository_key).await? {
        let res: GetRustRepositoriesResponse = redis_repo.get(&github_repository_key).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = RepositoriesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&params).await?;

    redis_repo.set(&github_repository_key, res.clone()).await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}

#[tracing::instrument(name = "Get repository good first issues", skip(state))]
pub async fn get_repository_good_first_issues(
    state: State<Arc<AppState>>,
    path: Path<GetRustRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetRustRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = GithubRedisRepository::new(&state.redis_pool).await?;
    let params = params.0;
    let path_params = path.0;
    let good_first_issues_key = GithubGoodFirstIssuesKeyGenerator {
        params: &params,
        path_params: &path_params,
    };

    if redis_repo.contains(&good_first_issues_key).await? {
        let res: GetRustRepositoryGoodFirstIssuesResponse =
            redis_repo.get(&good_first_issues_key).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = GoodFirstIssuesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&path_params, &params).await?;

    redis_repo.set(&good_first_issues_key, res.clone()).await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}
