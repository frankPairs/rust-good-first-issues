use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::{collections::HashMap, error::Error};

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ReqwestError(reqwest::Error),
    GithubSerializeResponseError(serde_json::Error),
    GithubAPIError(StatusCode, HeaderMap<HeaderValue>, String),
    ParseUrlError(url::ParseError),
}

impl std::fmt::Display for RustGoodFirstIssuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustGoodFirstIssuesError::ReqwestError(err) => {
                tracing::error!("ReqwestError url = {:?}", err.url());
                tracing::error!("ReqwestError status = {:?}", err.status());
                tracing::error!("ReqwestError source = {:?}", err.source());

                write!(f, "ReqwestError error: {}", err)
            }
            RustGoodFirstIssuesError::ParseUrlError(err) => {
                write!(f, "Parse url error: {}", err)
            }
            RustGoodFirstIssuesError::GithubAPIError(status_code, _, message) => {
                write!(f, "Github API error {}: {}", status_code, message)
            }
            RustGoodFirstIssuesError::GithubSerializeResponseError(err) => {
                write!(f, "Github response serialization error: {}", err)
            }
        }
    }
}

impl IntoResponse for RustGoodFirstIssuesError {
    fn into_response(self) -> Response {
        let err_message = self.to_string();

        tracing::error!("{}", err_message);

        match self {
            RustGoodFirstIssuesError::GithubAPIError(status_code, headers, _) => {
                let mut rate_limit_headers: HashMap<String, &HeaderValue> = HashMap::new();

                if let Some(value) = headers.get("retry-after") {
                    rate_limit_headers.insert(String::from("retry-after"), value);
                }

                if let Some(value) = headers.get("x-ratelimit-remaining") {
                    rate_limit_headers.insert(String::from("x-ratelimit-remaining"), value);
                }

                if let Some(value) = headers.get("x-ratelimit-reset") {
                    rate_limit_headers.insert(String::from("x-ratelimit-reset"), value);
                }

                if !rate_limit_headers.is_empty() {
                    return (StatusCode::TOO_MANY_REQUESTS, headers, err_message).into_response();
                }

                (status_code, err_message).into_response()
            }
            RustGoodFirstIssuesError::ReqwestError(err) => (
                err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                err_message,
            )
                .into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}
