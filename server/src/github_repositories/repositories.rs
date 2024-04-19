use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, JsonAsyncCommands};
use reqwest::{header, Client, Url};

use super::models::{
    GetRustRepositoriesParams, GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
    GithubIssue, GithubIssueAPI, GithubPullRequest, SearchGithubRepositoriesResponseAPI,
};
use crate::github_repositories::models::GithubRepository as GithubRepositoryModel;
use crate::{config::GithubSettings, errors::RustGoodFirstIssuesError};

const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;
const REDIS_EXPIRATION_TIME: i64 = 600;

#[derive(Debug)]
pub struct GithubRepositoriesHttpRepository {
    client: Client,
}

impl GithubRepositoriesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let github_token = settings.get_token();
        let mut headers = header::HeaderMap::new();

        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", github_token).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", GITHUB_API_VERSION.parse().unwrap());
        headers.insert("User-Agent", "frankPairs".parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(Self { client })
    }

    #[tracing::instrument(name = "Get Rust repositories from Github API", skip(self))]
    pub async fn get(
        &self,
        params: &GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let mut url = Url::parse(GITHUB_API_BASE_URL)
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?
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

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(RustGoodFirstIssuesError::GithubAPIError(
                response.status(),
                "Github API error while fetching repositories".to_string(),
            ));
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

#[derive(Debug)]
pub struct GithubRepositoriesRedisRepository<'a> {
    pub redis_conn: PooledConnection<'a, RedisConnectionManager>,
}

impl<'a> GithubRepositoriesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_conn = redis_pool
            .get()
            .await
            .map_err(RustGoodFirstIssuesError::RedisConnectionError)?;

        Ok(Self { redis_conn })
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

        self.redis_conn
            .json_set(&key, "$", &repositories_response)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_conn
            .expire(&key, REDIS_EXPIRATION_TIME)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(())
    }

    #[tracing::instrument(name = "Get Github repositories from Redis", skip(self))]
    pub async fn get(
        &mut self,
        params: &GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(params);

        let repositories_response: GetRustRepositoriesResponse = self
            .redis_conn
            .json_get(&key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(repositories_response)
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

        self.redis_conn
            .exists(&key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
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
pub struct GithubGoodFirstIssuesHttpRepository {
    client: Client,
}

impl GithubGoodFirstIssuesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let github_token = settings.get_token();
        let mut headers = header::HeaderMap::new();

        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", github_token).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", GITHUB_API_VERSION.parse().unwrap());
        headers.insert("User-Agent", "frankPairs".parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(Self { client })
    }
    #[tracing::instrument(
        name = "Get Rust repository good first issues from Github API",
        skip(self)
    )]
    pub async fn get(
        &self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let mut url = Url::parse(GITHUB_API_BASE_URL)
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?
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

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(RustGoodFirstIssuesError::GithubAPIError(
                response.status(),
                "Github API error while fetching issues".to_string(),
            ));
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
pub struct GithubGoodFirstIssuesRedisRepository<'a> {
    pub redis_conn: PooledConnection<'a, RedisConnectionManager>,
}

impl<'a> GithubGoodFirstIssuesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_conn = redis_pool
            .get()
            .await
            .map_err(RustGoodFirstIssuesError::RedisConnectionError)?;

        Ok(Self { redis_conn })
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

        self.redis_conn
            .json_set(&key, "$", &issues_response)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_conn
            .expire(&key, REDIS_EXPIRATION_TIME)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(())
    }

    #[tracing::instrument(name = "Get Github good first issues from Redis", skip(self))]
    pub async fn get(
        &mut self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let key = self.generate_repositories_key(path_params, params);

        let issues_response: GetRustRepositoryGoodFirstIssuesResponse = self
            .redis_conn
            .json_get(&key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(issues_response)
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

        self.redis_conn
            .exists(&key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
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
