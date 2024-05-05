use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ReqwestError(reqwest::Error),
    GithubAPIError(StatusCode, HeaderMap, String),
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
            RustGoodFirstIssuesError::GithubAPIError(status_code, _, message) => {
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
        let err_message = self.to_string();

        tracing::error!("{}", err_message);

        match self {
            // If there is a rate limit error from Github API, we return the rate limit headers, so we can avoid
            // making unnecessary requests. In that case, the status code will be 429 Too Many Requests.
            RustGoodFirstIssuesError::GithubAPIError(status_code, headers, _) => {
                let mut ratelimit_headers = HeaderMap::new();

                if let Some(retry_after) = headers.get("retry-after") {
                    ratelimit_headers.insert("retry-after", retry_after.clone());
                }

                if let Some(remaining) = headers.get("x-ratelimit-remaining") {
                    ratelimit_headers.insert("x-ratelimit-remaining", remaining.clone());
                }

                if let Some(reset) = headers.get("x-ratelimit-reset") {
                    ratelimit_headers.insert("x-ratelimit-reset", reset.clone());
                }

                if !ratelimit_headers.is_empty() {
                    return (StatusCode::TOO_MANY_REQUESTS, headers, err_message).into_response();
                }

                (status_code, err_message).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}
