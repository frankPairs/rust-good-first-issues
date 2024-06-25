use bb8::Pool;
use bb8_redis::RedisConnectionManager;

use super::client::RedisClient;
use crate::errors::RustGoodFirstIssuesError;
use crate::github::models::{
    GetRustRepositoriesParams, GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
};
use redis::{AsyncCommands, JsonAsyncCommands};

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
        let key = self.generate_key(params);

        self.redis_client
            .conn
            .json_set(&key, "$", &repositories_response)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_client
            .conn
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
        let key = self.generate_key(params);

        self.redis_client
            .conn
            .json_get(key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }

    #[tracing::instrument(
        name = "Check if there are Github repositories on Redis with a certain key",
        skip(self)
    )]
    pub async fn contains(
        &mut self,
        params: &GetRustRepositoriesParams,
    ) -> Result<bool, RustGoodFirstIssuesError> {
        let key = self.generate_key(params);

        self.redis_client
            .conn
            .exists(key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }

    fn generate_key(&self, params: &GetRustRepositoriesParams) -> String {
        format!(
            "github_repositories:rust:per_page={}&page={}",
            params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}

#[derive(Debug)]
pub struct GoodFirstIssuesRedisRepository<'a> {
    pub redis_client: RedisClient<'a>,
}

impl<'a> GoodFirstIssuesRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_client = RedisClient::new(redis_pool).await?;

        Ok(Self { redis_client })
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
        let key = self.generate_key(params, path_params);

        self.redis_client
            .conn
            .json_set(&key, "$", &issues_response)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_client
            .conn
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
        let key = self.generate_key(params, path_params);

        self.redis_client
            .conn
            .json_get(key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
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
        let key = self.generate_key(params, path_params);

        self.redis_client
            .conn
            .exists(key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }

    fn generate_key(
        &self,
        params: &GetRustRepositoryGoodFirstIssuesParams,
        path_params: &GetRustRepositoryGoodFirstIssuesPathParams,
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
