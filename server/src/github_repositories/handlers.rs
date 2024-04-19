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
use crate::github_repositories::repositories::GithubRepositoriesHttpRepository;

use super::models::{
    GetRustRepositoryGoodFirstIssuesParams, GetRustRepositoryGoodFirstIssuesPathParams,
};
use super::repositories::{
    GithubGoodFirstIssuesHttpRepository, GithubGoodFirstIssuesRedisRepository,
    GithubRepositoriesRedisRepository,
};

#[tracing::instrument(name = "Get rust repositories", skip(state))]
pub async fn get_rust_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetRustRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut repositories_redis_repo =
        GithubRepositoriesRedisRepository::new(&state.redis_pool).await?;
    let query_params = params.0;

    if repositories_redis_repo.contains(&query_params).await? {
        let res = repositories_redis_repo.get(&query_params).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let repositories_http_repo =
        GithubRepositoriesHttpRepository::new(state.github_settings.clone())?;
    let res = repositories_http_repo.get(&query_params).await?;

    repositories_redis_repo
        .set(&query_params, res.clone())
        .await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}

#[tracing::instrument(name = "Get repository good first issues", skip(state))]
pub async fn get_repository_good_first_issues(
    state: State<Arc<AppState>>,
    path: Path<GetRustRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetRustRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut issues_redis_repo =
        GithubGoodFirstIssuesRedisRepository::new(&state.redis_pool).await?;
    let query_params = params.0;
    let path_params = path.0;

    if issues_redis_repo
        .contains(&path_params, &query_params)
        .await?
    {
        let res = issues_redis_repo.get(&path_params, &query_params).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let issues_http_repo = GithubGoodFirstIssuesHttpRepository::new(state.github_settings.clone())?;

    let res = issues_http_repo.get(&path_params, &query_params).await?;

    issues_redis_repo
        .set(&path_params, &query_params, res.clone())
        .await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}
