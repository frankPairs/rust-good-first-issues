use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use std::fmt::Debug;

use super::{client::RedisClient, errors::RedisUtilsError};
use redis::{AsyncCommands, JsonAsyncCommands};

#[derive(Debug)]
pub struct RedisRepository<'a> {
    pub redis_client: RedisClient<'a>,
}

impl<'a> RedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RedisUtilsError> {
        let redis_client = RedisClient::new(redis_pool).await?;

        Ok(Self { redis_client })
    }

    // It stores data from Github on a Redis database. It will be expire after 10 minutes.
    #[tracing::instrument(name = "Stores data from Github on Redis", skip(self))]
    pub async fn set<V>(
        &mut self,
        key: String,
        value: V,
        expiration_time: Option<i64>,
    ) -> Result<(), RedisUtilsError>
    where
        V: Debug + serde::Serialize + Send + Sync,
    {
        self.redis_client
            .conn
            .json_set(&key, "$", &value)
            .await
            .map_err(RedisUtilsError::RedisError)?;

        if let Some(expiration_time) = expiration_time {
            self.redis_client
                .conn
                .expire(&key, expiration_time)
                .await
                .map_err(RedisUtilsError::RedisError)?;
        }

        Ok(())
    }

    #[tracing::instrument(name = "Get Github data from Redis", skip(self))]
    pub async fn get<R>(&mut self, key: String) -> Result<R, RedisUtilsError>
    where
        R: serde::de::DeserializeOwned + redis::FromRedisValue,
    {
        self.redis_client
            .conn
            .json_get(key, "$")
            .await
            .map_err(RedisUtilsError::RedisError)
    }

    #[tracing::instrument(name = "Check if a key exists on Redis", skip(self))]
    pub async fn contains(&mut self, key: String) -> Result<bool, RedisUtilsError> {
        self.redis_client
            .conn
            .exists(key)
            .await
            .map_err(RedisUtilsError::RedisError)
    }
}
