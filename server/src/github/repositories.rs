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
use crate::redis_repository::RedisRepository;
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

#[derive(Debug)]
pub struct RepositoriesRedisRepository<'a> {
    pub redis_repo: RedisRepository<'a>,
}

impl<'a> RepositoriesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_repo = RedisRepository::new(redis_pool).await?;

        Ok(Self { redis_repo })
    }

    // It stores some useful information about Github repositories. Filters of the HTTP request are used as a key.
    // This data will remain in Redis for 10 minutes.
    #[tracing::instrument(
        name = "Store Github repositories on Redis",
        skip(self, repositories_response)
    )]
    pub async fn set(
        &mut self,
        params: &GetRustRepositoriesParams,
        repositories_response: GetRustRepositoriesResponse,
    ) -> Result<(), RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(params);

        self.redis_repo.json_set(key, &repositories_response).await
    }

    #[tracing::instrument(name = "Get Github repositories from Redis", skip(self))]
    pub async fn get(
        &mut self,
        params: &GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(params);

        self.redis_repo.json_get(key).await
    }

    #[tracing::instrument(
        name = "Check if there are Github repositories on Redis with a certain key",
        skip(self)
    )]
    pub async fn contains(
        &mut self,
        params: &GetRustRepositoriesParams,
    ) -> Result<bool, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(params);

        self.redis_repo.contains(key).await
    }

    fn generate_repositories_key(&self, params: &GetRustRepositoriesParams) -> String {
        format!(
            "github_repositories:rust:per_page={}&page={}",
            params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}

#[derive(Debug)]
pub struct GoodFirstIssuesRedisRepository<'a> {
    pub redis_repo: RedisRepository<'a>,
}

impl<'a> GoodFirstIssuesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_repo = RedisRepository::new(redis_pool).await?;

        Ok(Self { redis_repo })
    }

    // It stores some useful information about Github good first issues. Filters of the HTTP request are used as a Redis key.
    // This data will remain in Redis for 10 minutes.
    #[tracing::instrument(
        name = "Store Github good first issues on Redis",
        skip(self, issues_response)
    )]
    pub async fn set(
        &mut self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
        issues_response: GetRustRepositoryGoodFirstIssuesResponse,
    ) -> Result<(), RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(path_params, params);

        self.redis_repo.json_set(key, &issues_response).await
    }

    #[tracing::instrument(name = "Get Github good first issues from Redis", skip(self))]
    pub async fn get(
        &mut self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(path_params, params);

        self.redis_repo.json_get(key).await
    }

    #[tracing::instrument(
        name = "Check if there are Github issues on Redis with a certain key",
        skip(self)
    )]
    pub async fn contains(
        &mut self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<bool, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(path_params, params);

        self.redis_repo.contains(key).await
    }

    fn generate_repositories_key(
        &self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> String {
        format!(
            "github_issues:rust:per_page={}&page={}&owner={}&repository_name={}&labels=good_first_issue",
            params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            params.page.unwrap_or(DEFAULT_PAGE),
            params.owner,
            path_params.repo
        )
    }
}