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
    GetRustRepositoryGoodFirstIssuesParams, GetRustRepositoryGoodFirstIssuesPathParams,
};
use super::repositories::{
    GoodFirstIssuesHttpRepository, GoodFirstIssuesRedisRepository, RepositoriesHttpRepository,
    RepositoriesRedisRepository,
};

#[tracing::instrument(name = "Get rust repositories", skip(state))]
pub async fn get_rust_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetRustRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = RepositoriesRedisRepository::new(&state.redis_pool).await?;
    let query_params = params.0;

    if redis_repo.contains(&query_params).await? {
        let res = redis_repo.get(&query_params).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = RepositoriesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&query_params).await?;

    redis_repo.set(&query_params, res.clone()).await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}

#[tracing::instrument(name = "Get repository good first issues", skip(state))]
pub async fn get_repository_good_first_issues(
    state: State<Arc<AppState>>,
    path: Path<GetRustRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetRustRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = GoodFirstIssuesRedisRepository::new(&state.redis_pool).await?;
    let query_params = params.0;
    let path_params = path.0;

    if redis_repo.contains(&path_params, &query_params).await? {
        let res = redis_repo.get(&path_params, &query_params).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = GoodFirstIssuesHttpRepository::new(state.github_settings.clone())?;
    let res = http_repo.get(&path_params, &query_params).await?;

    redis_repo
        .set(&path_params, &query_params, res.clone())
        .await?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}
