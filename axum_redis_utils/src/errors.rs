use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use redis::RedisError;

#[derive(Debug)]
pub enum RedisUtilsError {
    RedisError(RedisError),
    RedisConnectionError(bb8::RunError<redis::RedisError>),
}

impl std::fmt::Display for RedisUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisUtilsError::RedisError(err) => {
                let error_msg = format!("Redis error: {}", err);

                write!(f, "{}", error_msg)
            }

            RedisUtilsError::RedisConnectionError(err) => {
                let error_msg = format!("Redis connection error: {}", err);

                write!(f, "{}", error_msg)
            }
        }
    }
}

impl IntoResponse for RedisUtilsError {
    fn into_response(self) -> Response {
        let err_message = self.to_string();

        (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response()
    }
}
