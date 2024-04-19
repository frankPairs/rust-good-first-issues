use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ReqwestError(reqwest::Error),
    GithubAPIError(StatusCode, String),
    ParseUrlError(url::ParseError),
    RedisError(redis::RedisError),
    RedisConnectionError(bb8::RunError<redis::RedisError>),
}

impl std::fmt::Display for RustGoodFirstIssuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustGoodFirstIssuesError::ReqwestError(err) => {
                write!(f, "External API request error: {}", err)
            }
            RustGoodFirstIssuesError::ParseUrlError(err) => {
                write!(f, "Parse url error: {}", err)
            }
            RustGoodFirstIssuesError::GithubAPIError(status_code, message) => {
                write!(f, "Github API error {}: {}", status_code, message)
            }
            RustGoodFirstIssuesError::RedisError(err) => {
                write!(f, "Redis error: {}", err)
            }
            RustGoodFirstIssuesError::RedisConnectionError(err) => {
                write!(f, "Redis connection error: {}", err)
            }
        }
    }
}

impl IntoResponse for RustGoodFirstIssuesError {
    fn into_response(self) -> Response {
        match self {
            RustGoodFirstIssuesError::ReqwestError(err) => {
                tracing::error!("External API request error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            RustGoodFirstIssuesError::ParseUrlError(err) => {
                tracing::error!("Parse url error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            RustGoodFirstIssuesError::GithubAPIError(status_code, message) => {
                tracing::error!("Github API error {}:  {}", status_code, message);

                (status_code, message).into_response()
            }
            RustGoodFirstIssuesError::RedisError(err) => {
                tracing::error!("Redis error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            RustGoodFirstIssuesError::RedisConnectionError(err) => {
                tracing::error!("Redis connection error:  {}", err);

                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
        }
    }
}
