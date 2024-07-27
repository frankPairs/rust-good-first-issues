use axum::{async_trait, extract::FromRequestParts};

#[async_trait]
pub trait RedisKeyGeneratorExtractor<S: Send + Sync>: FromRequestParts<S> {
    fn generate_key(&self) -> String;
}
