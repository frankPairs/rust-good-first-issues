use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, FromRedisValue, JsonAsyncCommands};
use serde::{de::DeserializeOwned, Serialize};

use crate::errors::RustGoodFirstIssuesError;

const REDIS_EXPIRATION_TIME: i64 = 600;

#[derive(Debug)]
pub struct RedisRepository<'a> {
    pub redis_conn: PooledConnection<'a, RedisConnectionManager>,
}

impl<'a> RedisRepository<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let redis_conn = redis_pool
            .get()
            .await
            .map_err(RustGoodFirstIssuesError::RedisConnectionError)?;

        Ok(Self { redis_conn })
    }

    // Sets data on Redis using a key. This data will remain in Redis for 10 minutes.
    #[tracing::instrument(name = "Store information on Redis using a key", skip(self, data))]
    pub async fn json_set<D: Serialize + Sync + Send>(
        &mut self,
        key: String,
        data: D,
    ) -> Result<(), RustGoodFirstIssuesError> {
        self.redis_conn
            .json_set(&key, "$", &data)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        self.redis_conn
            .expire(&key, REDIS_EXPIRATION_TIME)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(())
    }

    #[tracing::instrument(name = "Get data from Redis", skip(self))]
    pub async fn json_get<D: DeserializeOwned + FromRedisValue>(
        &mut self,
        key: String,
    ) -> Result<D, RustGoodFirstIssuesError> {
        let data: D = self
            .redis_conn
            .json_get(&key, "$")
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)?;

        Ok(data)
    }

    #[tracing::instrument(name = "Check if data exists on Redis with a certain key", skip(self))]
    pub async fn contains(&mut self, key: String) -> Result<bool, RustGoodFirstIssuesError> {
        self.redis_conn
            .exists(&key)
            .await
            .map_err(RustGoodFirstIssuesError::RedisError)
    }
}
