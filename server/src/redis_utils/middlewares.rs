use axum::{
    body::{Body, Bytes},
    extract::{Json, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use std::fmt::Debug;
use std::sync::Arc;

use crate::state::AppState;

use super::{
    errors::RedisUtilsError, extractors::RedisKeyGeneratorExtractor, repositories::RedisRepository,
};

const REDIS_EXPIRATION_TIME: i64 = 600;

pub async fn with_redis_cache<
    K: RedisKeyGeneratorExtractor<Arc<AppState>>,
    R: serde::Serialize + serde::de::DeserializeOwned + redis::FromRedisValue + Debug + Send + Sync,
>(
    State(state): State<Arc<AppState>>,
    redis_key_generator: K,
    request: Request,
    next: Next,
) -> Result<Response, RedisUtilsError> {
    let resource_key = redis_key_generator.generate_key();

    let mut redis_repo = RedisRepository::new(&state.redis_pool).await?;

    if redis_repo.contains(resource_key.clone()).await? {
        let res: R = redis_repo.get(resource_key.clone()).await?;

        return Ok((StatusCode::OK, Json(res)).into_response());
    }

    let res = next.run(request).await;

    let (parts, body) = res.into_parts();

    let bytes = save_response_body_into_redis::<R>(body, &mut redis_repo, resource_key).await?;

    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn save_response_body_into_redis<'a, R>(
    body: Body,
    redis_repo: &mut RedisRepository<'a>,
    resource_key: String,
) -> Result<Bytes, RedisUtilsError>
where
    R: serde::Serialize + serde::de::DeserializeOwned + redis::FromRedisValue + Debug + Send + Sync,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(RedisUtilsError::BadRequest(err.to_string()));
        }
    };
    let res_json_str = match String::from_utf8(bytes.to_vec()) {
        Ok(json_str) => json_str,
        Err(err) => {
            return Err(RedisUtilsError::BadRequest(err.to_string()));
        }
    };
    let res_body: R =
        serde_json::from_str(&res_json_str).map_err(RedisUtilsError::SerdeJsonError)?;

    redis_repo
        .set(resource_key, res_body, Some(REDIS_EXPIRATION_TIME))
        .await?;

    Ok(bytes)
}
