use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::error::Error;

const GITHUB_RATE_LIMIT_HEADERS: [&str; 3] =
    ["retry-after", "x-ratelimit-remaining", "x-ratelimit-reset"];

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    Reqwest(reqwest::Error),
    GithubAPI(StatusCode, HeaderMap<HeaderValue>, String),
    ParseUrl(url::ParseError),
}

impl std::fmt::Display for RustGoodFirstIssuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustGoodFirstIssuesError::Reqwest(err) => {
                tracing::error!("ReqwestError url = {:?}", err.url());
                tracing::error!("ReqwestError status = {:?}", err.status());
                tracing::error!("ReqwestError source = {:?}", err.source());

                write!(f, "ReqwestError error: {}", err)
            }
            RustGoodFirstIssuesError::ParseUrl(err) => {
                write!(f, "Parse url error: {}", err)
            }
            RustGoodFirstIssuesError::GithubAPI(status_code, _, message) => {
                write!(f, "Github API error {}: {}", status_code, message)
            }
        }
    }
}

impl IntoResponse for RustGoodFirstIssuesError {
    fn into_response(self) -> Response {
        let err_message = self.to_string();

        tracing::error!("{}", err_message);

        match self {
            RustGoodFirstIssuesError::GithubAPI(status_code, headers, _) => {
                let rate_limit_headers = HeaderMap::from_iter(
                    headers
                        .iter()
                        .filter(|(name, _)| GITHUB_RATE_LIMIT_HEADERS.contains(&name.as_str()))
                        .map(|(name, value)| (name.clone(), value.clone())),
                );

                // Just returning the rate limit headers from Github API
                (status_code, rate_limit_headers, err_message).into_response()
            }
            RustGoodFirstIssuesError::Reqwest(err) => (
                err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                err_message,
            )
                .into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}
