use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use itertools::{sorted, Itertools};

const REDIS_KEY_DELIMITER: &str = ":";

pub struct ExtractRedisKey(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractRedisKey
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let url = parts.uri.clone();
        let formatted_path = url.path().to_string().replace("/", REDIS_KEY_DELIMITER);
        let query_params = match url.query() {
            Some(query) => query,
            None => "",
        };
        let sorted_params = sorted(query_params.split("&")).join(REDIS_KEY_DELIMITER);

        let redis_key = format!("{}{}", formatted_path, sorted_params).replacen(":", "", 1);

        Ok(ExtractRedisKey(redis_key))
    }
}
