use bb8::Pool;
use bb8_redis::RedisConnectionManager;

use super::client::RedisClient;
use crate::errors::RustGoodFirstIssuesError;
use crate::github::models::{
    GetRustRepositoriesParams, GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
};

const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;
const REDIS_EXPIRATION_TIME: i64 = 600;

#[derive(Debug)]
pub struct RepositoriesRedisRepository<'a> {
    pub redis_client: RedisClient<'a>,
}

impl<'a> RepositoriesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_client = RedisClient::new(redis_pool).await?;

        Ok(Self { redis_client })
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
        let key = RepositoriesRedisKeyGenerator { params };

        self.redis_client
            .conn
            .json_set(key, &repositories_response, Some(REDIS_EXPIRATION_TIME))
            .await
    }

    #[tracing::instrument(name = "Get Github repositories from Redis", skip(self))]
    pub async fn get(
        &mut self,
        params: &GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let key = RepositoriesRedisKeyGenerator { params };

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
        let key = RepositoriesRedisKeyGenerator { params };

        self.redis_repo.contains(key).await
    }
}

#[derive(Debug)]
pub struct GoodFirstIssuesRedisRepository<'a> {
    pub redis_repo: RedisClient<'a>,
}

impl<'a> GoodFirstIssuesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_repo = RedisClient::new(redis_pool).await?;

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
        let key = GoodFirstIssuesRedisKeyGenerator {
            path_params,
            params,
        };

        self.redis_repo
            .json_set(key, &issues_response, Some(REDIS_EXPIRATION_TIME))
            .await
    }

    #[tracing::instrument(name = "Get Github good first issues from Redis", skip(self))]
    pub async fn get(
        &mut self,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
        params: &GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let key = GoodFirstIssuesRedisKeyGenerator {
            path_params,
            params,
        };

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
        let key = GoodFirstIssuesRedisKeyGenerator {
            path_params,
            params,
        };

        self.redis_repo.contains(key).await
    }
}

#[derive(Debug)]
pub struct RepositoriesRedisKeyGenerator<'a> {
    pub params: &'a GetRustRepositoriesParams,
}

impl<'a> RedisKeyGenerator for RepositoriesRedisKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github_repositories:rust:per_page={}&page={}",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}

#[derive(Debug)]
struct GoodFirstIssuesRedisKeyGenerator<'a> {
    path_params: &'a GetRustRepositoryGoodFirstIssuesPathParams,
    params: &'a GetRustRepositoryGoodFirstIssuesParams,
}

impl<'a> RedisKeyGenerator for GoodFirstIssuesRedisKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github_issues:rust:per_page={}&page={}&owner={}&repository_name={}&labels=good_first_issue",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE),
            self.params.owner,
            self.path_params.repo
        )
    }
}

pub struct RateLimitKeyGenerator {
    pub key: String,
}

impl<'a> RedisKeyGenerator for RateLimitKeyGenerator {
    fn generate_key(&self) -> String {
        format!("errors:rate_limit:{}", self.key)
    }
}
