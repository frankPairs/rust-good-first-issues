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
    GetGithubRepositoriesResponse, GetGithubRepositoryGoodFirstIssuesParams,
    GetGithubRepositoryGoodFirstIssuesPathParams, GetGithubRepositoryGoodFirstIssuesResponse,
};
use super::repositories::http::repositories::{
    GithubGoodFirstIssuesHttpRepository, GithubRepositoriesHttpRepository,
};
use super::repositories::redis::repositories::{
    GithubGoodFirstIssuesKeyGenerator, GithubRateLimitErrorKeyGenerator, GithubRedisRepository,
    GithubRepositoriesKeyGenerator,
};

#[tracing::instrument(name = "Get Github repositories handler", skip(state))]
pub async fn get_github_repositories(
    state: State<Arc<AppState>>,
    params: Query<GetGithubRepositoriesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = GithubRedisRepository::new(&state.redis_pool).await?;
    let params = params.0;
    let github_repositories_key = GithubRepositoriesKeyGenerator { params: &params };
    let rate_limit_key = GithubRateLimitErrorKeyGenerator {
        src_key_generator: &github_repositories_key,
    };

    if redis_repo.contains(&rate_limit_key).await? {
        return Ok((StatusCode::TOO_MANY_REQUESTS).into_response());
    }

    if redis_repo.contains(&github_repositories_key).await? {
        let res: GetGithubRepositoriesResponse = redis_repo.get(&github_repositories_key).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = GithubRepositoriesHttpRepository::new(state.github_settings.clone())?;
    let res_result = http_repo.get(&params).await;

    match res_result {
        Ok(res) => {
            redis_repo
                .set(&github_repositories_key, res.clone(), None)
                .await?;

            return Ok((StatusCode::OK, Json(res)).into_response());
        }
        Err(err) => match err {
            RustGoodFirstIssuesError::GithubRateLimitError(_, err_payload) => {
                redis_repo
                    .set(
                        &rate_limit_key,
                        err_payload,
                        Some(err_payload.get_expiration_time()),
                    )
                    .await?;

                Ok(err.into_response())
            }
            err => Ok(err.into_response()),
        },
    }
}

#[tracing::instrument(name = "Get Github repository good first issues", skip(state))]
pub async fn get_github_repository_good_first_issues(
    state: State<Arc<AppState>>,
    path: Path<GetGithubRepositoryGoodFirstIssuesPathParams>,
    params: Query<GetGithubRepositoryGoodFirstIssuesParams>,
) -> Result<Response, RustGoodFirstIssuesError> {
    let mut redis_repo = GithubRedisRepository::new(&state.redis_pool).await?;
    let params = params.0;
    let path_params = path.0;
    let good_first_issues_key = GithubGoodFirstIssuesKeyGenerator {
        params: &params,
        path_params: &path_params,
    };
    let rate_limit_key = GithubRateLimitErrorKeyGenerator {
        src_key_generator: &good_first_issues_key,
    };

    if redis_repo.contains(&rate_limit_key).await? {
        return Ok((StatusCode::TOO_MANY_REQUESTS).into_response());
    }

    if redis_repo.contains(&good_first_issues_key).await? {
        let res: GetGithubRepositoryGoodFirstIssuesResponse =
            redis_repo.get(&good_first_issues_key).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let http_repo = GithubGoodFirstIssuesHttpRepository::new(state.github_settings.clone())?;
    let res_result = http_repo.get(&path_params, &params).await;

    match res_result {
        Ok(res) => {
            redis_repo
                .set(&good_first_issues_key, res.clone(), None)
                .await?;

            return Ok((StatusCode::OK, Json(res)).into_response());
        }
        Err(err) => match err {
            RustGoodFirstIssuesError::GithubRateLimitError(_, err_payload) => {
                redis_repo
                    .set(
                        &rate_limit_key,
                        err_payload,
                        Some(err_payload.get_expiration_time()),
                    )
                    .await?;

                Ok(err.into_response())
            }
            err => Ok(err.into_response()),
        },
    }
}
