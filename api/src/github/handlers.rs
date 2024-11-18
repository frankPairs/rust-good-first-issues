use axum::extract::Path;
use axum::response::Response;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use redis::{AsyncCommands, JsonAsyncCommands};

use std::sync::Arc;

use crate::errors::RustGoodFirstIssuesError;
use crate::extractors::ExtractRedisKey;
use crate::github::models::GetGithubRepositoriesParams;
use crate::state::AppState;

use super::client::GithubHttpClient;
use super::models::{
    GetGithubRepositoriesResponse, GetGithubRepositoryGoodFirstIssuesParams,
    GetGithubRepositoryGoodFirstIssuesPathParams, GetGithubRepositoryGoodFirstIssuesResponse,
};

const GITHUB_REDIS_EXPIRATION_TIME: i64 = 600;

#[tracing::instrument(name = "Get Github repositories handler", skip(state, redis_key))]
pub async fn get_repositories(
    state: State<Arc<AppState>>,
    ExtractRedisKey(redis_key): ExtractRedisKey,
    params: Query<GetGithubRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_conn = state
        .redis_pool
        .get()
        .await
        .map_err(RustGoodFirstIssuesError::RedisConnection)?;

    if redis_conn
        .exists(&redis_key)
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?
    {
        let res = redis_conn
            .json_get::<&str, &str, GetGithubRepositoriesResponse>(&redis_key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::Redis)?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let params = params.0;
    let github_client = GithubHttpClient::new(state.github_settings.clone())?;
    let res = github_client.get_rust_repositories(&params).await?;

    redis_conn
        .json_set::<&str, &str, GetGithubRepositoriesResponse, ()>(&redis_key, "$", &res)
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?;

    redis_conn
        .expire::<&str, ()>(&redis_key, GITHUB_REDIS_EXPIRATION_TIME)
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}

#[tracing::instrument(name = "Get Github repository good first issues", skip(state))]
pub async fn get_repository_good_first_issues(
    state: State<Arc<AppState>>,
    ExtractRedisKey(redis_key): ExtractRedisKey,
    path: Path<GetGithubRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetGithubRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let params = params.0;
    let path_params = path.0;

    let mut redis_conn = state
        .redis_pool
        .get()
        .await
        .map_err(RustGoodFirstIssuesError::RedisConnection)?;

    if redis_conn
        .exists(&redis_key)
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?
    {
        let res = redis_conn
            .json_get::<&str, &str, GetGithubRepositoryGoodFirstIssuesResponse>(&redis_key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::Redis)?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let github_client = GithubHttpClient::new(state.github_settings.clone())?;
    let res = github_client
        .get_repository_good_first_issues(&path_params, &params)
        .await?;

    redis_conn
        .json_set::<&str, &str, GetGithubRepositoryGoodFirstIssuesResponse, ()>(
            &redis_key, "$", &res,
        )
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?;

    redis_conn
        .expire::<&str, ()>(&redis_key, GITHUB_REDIS_EXPIRATION_TIME)
        .await
        .map_err(RustGoodFirstIssuesError::Redis)?;

    return Ok((StatusCode::OK, Json(res)).into_response());
}
