use axum::{async_trait, http::request::Parts};

use crate::errors::RustGoodFirstIssuesError;

#[async_trait]
pub trait RedisKeyGenerator: Sized {
    async fn from_request_parts(parts: &mut Parts) -> Result<Self, RustGoodFirstIssuesError>;

    fn generate_key(&self) -> String;
}
