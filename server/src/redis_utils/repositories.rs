use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, JsonAsyncCommands};
use std::fmt::Debug;

use super::errors::RedisUtilsError;

#[derive(Debug)]
pub struct RedisRepository<'a> {
    pub conn: PooledConnection<'a, RedisConnectionManager>,
}

impl<'a> RedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RedisUtilsError> {
        let conn = redis_pool
            .get()
            .await
            .map_err(RedisUtilsError::RedisConnectionError)?;

        Ok(Self { conn })
    }

    #[tracing::instrument(
        name = "Stores data on Redis database",
        skip(self, value, expiration_time)
    )]
    pub async fn set<V>(
        &mut self,
        key: String,
        value: V,
        expiration_time: Option<i64>,
    ) -> Result<(), RedisUtilsError>
    where
        V: Debug + serde::Serialize + Send + Sync,
    {
        self.conn
            .json_set(&key, "$", &value)
            .await
            .map_err(RedisUtilsError::RedisError)?;

        if let Some(expiration_time) = expiration_time {
            self.conn
                .expire(&key, expiration_time)
                .await
                .map_err(RedisUtilsError::RedisError)?;
        }

        Ok(())
    }

    #[tracing::instrument(name = "Get data from Redis database", skip(self))]
    pub async fn get<R>(&mut self, key: String) -> Result<R, RedisUtilsError>
    where
        R: serde::de::DeserializeOwned + redis::FromRedisValue,
    {
        self.conn
            .json_get(key, "$")
            .await
            .map_err(RedisUtilsError::RedisError)
    }

    #[tracing::instrument(name = "Check if a key exists on Redis database", skip(self))]
    pub async fn contains(&mut self, key: String) -> Result<bool, RedisUtilsError> {
        self.conn
            .exists(key)
            .await
            .map_err(RedisUtilsError::RedisError)
    }
}
