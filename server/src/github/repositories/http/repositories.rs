use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use reqwest::Client;

use super::http_client::GithubHttpClient;
use super::models::{
    GetRustRepositoriesParams, GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
    GithubIssue, GithubIssueAPI, GithubPullRequest, SearchGithubRepositoriesResponseAPI,
};
use crate::github::models::GithubRepository as GithubRepositoryModel;
use crate::redis_client::{RedisClient, RedisKeyGenerator};
use crate::{config::GithubSettings, errors::RustGoodFirstIssuesError};

const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;
const REDIS_EXPIRATION_TIME: i64 = 600;

pub struct RepositoriesHttpRepository {
    http_client: GithubHttpClient,
}

impl RepositoriesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let github_token = settings.get_token();
        let http_client = GithubHttpClient::new(github_token)?;

        Ok(Self { http_client })
    }

    #[tracing::instrument(name = "Get Rust repositories from Github API", skip(self))]
    pub async fn get(
        &self,
        params: &GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let client: &Client = self.http_client.get_client();
        let mut url = self
            .http_client
            .get_base_url()?
            .join("/search/repositories?")
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?;

        url.query_pairs_mut()
            .append_pair("q", "language:rust")
            .append_pair("sort", "help-wanted-issues")
            .append_pair("order", "desc")
            .append_pair(
                "per_page",
                &params.per_page.unwrap_or(DEFAULT_PER_PAGE).to_string(),
            )
            .append_pair("page", &params.page.unwrap_or(DEFAULT_PAGE).to_string());

        let response = client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(self.http_client.try_into_error(response).await);
        }

        let json: SearchGithubRepositoriesResponseAPI = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetRustRepositoriesResponse {
            total_count: json.total_count,
            items: json
                .items
                .into_iter()
                .map(|repo| GithubRepositoryModel {
                    id: repo.id,
                    url: repo.html_url,
                    name: repo.full_name,
                    private: repo.private,
                    avatar_url: repo.owner.avatar_url,
                    description: repo.description,
                    stars_count: repo.stargazers_count,
                    open_issues_count: repo.open_issues_count,
                    has_issues: repo.has_issues,
                    license: repo.license.name,
                })
                .collect(),
        })
    }
}

pub struct GoodFirstIssuesHttpRepository {
    http_client: GithubHttpClient,
}

impl GoodFirstIssuesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let github_token = settings.get_token();
        let http_client = GithubHttpClient::new(github_token)?;

        Ok(Self { http_client })
    }

    pub async fn get(
        &self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let client: &Client = self.http_client.get_client();
        let mut url = self
            .http_client
            .get_base_url()?
            .join(&format!(
                "/repos/{}/{}/issues?",
                params.owner, path_params.repo
            ))
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?;

        url.query_pairs_mut()
            .append_pair("labels", "good first issue")
            .append_pair("sort", "updated")
            .append_pair("direction", "desc")
            .append_pair(
                "per_page",
                &params.per_page.unwrap_or(DEFAULT_PER_PAGE).to_string(),
            )
            .append_pair("page", &params.page.unwrap_or(DEFAULT_PAGE).to_string());

        let response = client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(self.http_client.try_into_error(response).await);
        }

        let json: Vec<GithubIssueAPI> = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetRustRepositoryGoodFirstIssuesResponse {
            items: json
                .into_iter()
                .map(|issue| GithubIssue {
                    id: issue.id,
                    body: issue.body,
                    description: issue.description,
                    state: issue.state,
                    title: issue.title,
                    url: issue.html_url,
                    pull_request: if let Some(pull_request) = issue.pull_request {
                        Some(GithubPullRequest {
                            url: pull_request.html_url,
                        })
                    } else {
                        None
                    },
                })
                .collect(),
        })
    }
}