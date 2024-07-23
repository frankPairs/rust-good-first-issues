use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ReqwestError(reqwest::Error),
    GithubAPIError(StatusCode, String),
    GithubRateLimitError(String, RateLimitErrorPayload),
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
            RustGoodFirstIssuesError::GithubRateLimitError(message, _) => {
                write!(f, "Github rate limit error: {}", message)
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
        let err_message = self.to_string();

        tracing::error!("{}", err_message);

        match self {
            RustGoodFirstIssuesError::GithubAPIError(status_code, _) => {
                (status_code, err_message).into_response()
            }
            RustGoodFirstIssuesError::GithubRateLimitError(_, _) => {
                (StatusCode::TOO_MANY_REQUESTS, err_message).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}

#[derive(Debug)]
pub struct RateLimitErrorPayload {
    pub retry_after: Option<i32>,
    pub ratelimit_remaining: Option<i32>,
    pub ratelimit_reset: Option<i32>,
}

impl RateLimitErrorPayload {
    pub fn is_empty(&self) -> bool {
        return self.retry_after.is_none()
            || self.ratelimit_remaining.is_none() && self.ratelimit_reset.is_none();
    }

    pub fn from_response_headers(headers: &HeaderMap) -> Self {
        let mut retry_after: Option<i32> = None;
        let mut ratelimit_remaining: Option<i32> = None;
        let mut ratelimit_reset: Option<i32> = None;

        if let Some(value) = headers.get("retry-after") {
            let parsed_value = value.to_str().unwrap_or("");

            retry_after = match String::from(parsed_value).parse::<i32>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-remaining") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_remaining = match String::from(parsed_value).parse::<i32>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-reset") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_reset = match String::from(parsed_value).parse::<i32>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        RateLimitErrorPayload {
            ratelimit_remaining,
            ratelimit_reset,
            retry_after,
        }
    }
}
