/// ExtractRedisKey is an Axum extractor that extracts a Redis key from the request path and query parameters. This extractor is used within the RedisCacheLayer to generate a key for the cache.
///
/// The key is constructed by concatenating the path and query parameters. Query parameters are sorted alphabetically in order to ensure the same query parameters result in the same key,
/// independently of the order they were provided in the request.
///
/// It separates the path segments and query parameters by a colon (:).
///
/// For example, given the following request:
///
/// GET https://domain.com/api/v1/users?name=John&age=30
///
/// The key generated would be:
///
/// api:v1:users:age=30:name=John
///
/// The path used depend on where the RedisCacheLayer is mounted. For example, if the RedisCacheLayer is mounted at users layer, the key would be:
///
/// users:age=30:name=John
///
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
        let mut formatted_path = url.path().to_string().replace("/", REDIS_KEY_DELIMITER);
        let query_params = match url.query() {
            Some(query) => query,
            None => "",
        };
        let mut sorted_params = sorted(query_params.split("&")).join(REDIS_KEY_DELIMITER);

        // Removes the first colon from the formatted_path and sorted_params if they exist. This is done to ensure the key is consistent.
        // For example, if the path is /api/v1/users and the query parameters are name=John&age=30, the key should be api:v1:users:age=30:name=John
        if Some(':') == formatted_path.chars().next() {
            formatted_path = formatted_path.chars().skip(1).collect::<String>();
        }

        if Some(':') == sorted_params.chars().next() {
            sorted_params = sorted_params.chars().skip(1).collect::<String>();
        }

        let redis_key = format!("{}:{}", formatted_path, sorted_params);

        Ok(ExtractRedisKey(redis_key))
    }
}
