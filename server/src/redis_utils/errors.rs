use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum RedisUtilsError {
    RedisError(redis::RedisError),
    RedisConnectionError(bb8::RunError<redis::RedisError>),
}

impl std::fmt::Display for RedisUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisUtilsError::RedisError(err) => {
                write!(f, "Redis error: {}", err)
            }

            RedisUtilsError::RedisConnectionError(err) => {
                write!(f, "Redis connection error: {}", err)
            }
        }
    }
}

impl IntoResponse for RedisUtilsError {
    fn into_response(self) -> Response {
        let err_message = self.to_string();

        tracing::error!("{}", err_message);

        (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response()
    }
}
