use std::fmt::Debug;

use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;

use crate::errors::RustGoodFirstIssuesError;

#[derive(Debug)]
pub struct RedisClient<'a> {
    pub conn: PooledConnection<'a, RedisConnectionManager>,
}

impl<'a> RedisClient<'a> {
    pub async fn new(
        redis_pool: &'a Pool<RedisConnectionManager>,
    ) -> Result<Self, RustGoodFirstIssuesError> {
        let conn = redis_pool
            .get()
            .await
            .map_err(RustGoodFirstIssuesError::RedisConnectionError)?;

        Ok(Self { conn })
    }
}
