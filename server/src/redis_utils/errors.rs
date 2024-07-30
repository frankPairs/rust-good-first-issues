use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum RedisUtilsError {
    RedisError(redis::RedisError),
    RedisConnectionError(bb8::RunError<redis::RedisError>),
    BadRequest(String),
    SerdeJsonError(serde_json::Error),
}

impl std::fmt::Display for RedisUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisUtilsError::RedisError(err) => {
                write!(f, "Redis error: {}", err)
            }
            RedisUtilsError::BadRequest(err) => {
                write!(f, "Bad request: {}", err)
            }
            RedisUtilsError::SerdeJsonError(err) => {
                write!(f, "Serde JSON conversion error: {}", err)
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

        match self {
            RedisUtilsError::SerdeJsonError(_) | RedisUtilsError::BadRequest(_) => {
                (StatusCode::BAD_REQUEST, err_message).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}
