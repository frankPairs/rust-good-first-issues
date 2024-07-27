use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

use crate::errors::RustGoodFirstIssuesError;

use super::models::RedisKeyGenerator;

pub struct ExtractRedisKeyGenerator<K>(pub K);

#[async_trait]
impl<K, S> FromRequestParts<S> for ExtractRedisKeyGenerator<K>
where
    K: RedisKeyGenerator,
    S: Send + Sync,
{
    type Rejection = RustGoodFirstIssuesError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let key_generator = K::from_request_parts(parts).await?;

        Ok(Self(key_generator))
    }
}
