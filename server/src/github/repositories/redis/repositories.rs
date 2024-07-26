use std::fmt::Debug;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;

use super::client::RedisClient;
use crate::errors::RustGoodFirstIssuesError;
use crate::github::models::{
    GetGithubRepositoriesParams, GetGithubRepositoryGoodFirstIssuesParams,
    GetGithubRepositoryGoodFirstIssuesPathParams,
};
use redis::{AsyncCommands, JsonAsyncCommands};

const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;
// Expiration time is represented in seconds
const REDIS_EXPIRATION_TIME: i64 = 600;

trait RedisKeyGenerator {
    fn generate_key(&self) -> String;
}

#[derive(Debug)]
pub struct GithubRepositoriesKeyGenerator<'a> {
    pub params: &'a GetGithubRepositoriesParams,
}

impl<'a> RedisKeyGenerator for GithubRepositoriesKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github:repositories:rust:per_page={}&page={}",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}

#[derive(Debug)]
pub struct GithubRepositoriesRateLimitKeyGenerator<'a> {
    pub params: &'a GetGithubRepositoriesParams,
}

impl<'a> RedisKeyGenerator for GithubRepositoriesRateLimitKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github:repositories:rate_limit:per_page={}&page={}",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}

#[derive(Debug)]
pub struct GithubGoodFirstIssuesKeyGenerator<'a> {
    pub path_params: &'a GetGithubRepositoryGoodFirstIssuesPathParams,
    pub params: &'a GetGithubRepositoryGoodFirstIssuesParams,
}

impl<'a> RedisKeyGenerator for GithubGoodFirstIssuesKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github:issues:rust:per_page={}&page={}&owner={}&repository_name={}&labels=good_first_issue",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE),
            self.params.owner,
            self.path_params.repo
        )
    }
}

#[derive(Debug)]
pub struct GithubGoodFirstIssuesRateLimitKeyGenerator<'a> {
    pub path_params: &'a GetGithubRepositoryGoodFirstIssuesPathParams,
    pub params: &'a GetGithubRepositoryGoodFirstIssuesParams,
}

impl<'a> RedisKeyGenerator for GithubGoodFirstIssuesRateLimitKeyGenerator<'a> {
    fn generate_key(&self) -> String {
        format!(
            "github:issues:rate_limig:per_page={}&page={}&owner={}&repository_name={}&labels=good_first_issue",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE),
            self.params.owner,
            self.path_params.repo
        )
    }
}

#[derive(Debug)]
pub struct GithubRedisRepository<'a> {
    pub redis_client: RedisClient<'a>,
}

impl<'a> GithubRedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_client = RedisClient::new(redis_pool).await?;

        Ok(Self { redis_client })
    }

    // It stores data from Github on a Redis database. It will be expire after 10 minutes.
    #[tracing::instrument(name = "Stores data from Github on Redis", skip(self))]
    pub async fn set<K, V>(
        &mut self,
        key_generator: &K,
        value: V,
        expiration_time: Option<i64>,
    ) -> Result<(), RustGoodFirstIssuesError>
    where
        K: Debug + RedisKeyGenerator,
        V: Debug + serde::Serialize + Send + Sync,
    {
        let key = key_generator.generate_key();

        self.redis_client
            .conn
            .json_set(&key, "$", &value)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_client
            .conn
            .expire(&key, expiration_time.unwrap_or(REDIS_EXPIRATION_TIME))
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(())
    }

    #[tracing::instrument(name = "Get Github data from Redis", skip(self))]
    pub async fn get<K, R>(&mut self, key_generator: &K) -> Result<R, RustGoodFirstIssuesError>
    where
        K: Debug + RedisKeyGenerator,
        R: serde::de::DeserializeOwned + redis::FromRedisValue,
    {
        let key = key_generator.generate_key();

        self.redis_client
            .conn
            .json_get(key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }

    #[tracing::instrument(name = "Check if a key exists on Redis", skip(self))]
    pub async fn contains<K>(&mut self, key_generator: &K) -> Result<bool, RustGoodFirstIssuesError>
    where
        K: Debug + RedisKeyGenerator,
    {
        let key = key_generator.generate_key();

        self.redis_client
            .conn
            .exists(key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }
}
