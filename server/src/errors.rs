use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::error::Error;

#[derive(Debug)]
pub enum RustGoodFirstIssuesError {
    ReqwestError(reqwest::Error),
    GithubAPIError(StatusCode, String),
    GithubRateLimitError(String, RateLimitErrorPayload),
    ParseUrlError(url::ParseError),
}

impl std::fmt::Display for RustGoodFirstIssuesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustGoodFirstIssuesError::ReqwestError(err) => {
                tracing::error!("ReqwestError url = {:?}", err.url());
                tracing::error!("ReqwestError status = {:?}", err.status());
                tracing::error!("ReqwestError source = {:?}", err.source());

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
            RustGoodFirstIssuesError::ReqwestError(err) => (
                err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                err_message,
            )
                .into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, err_message).into_response(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RateLimitErrorPayload {
    // The time in seconds that you should wait before making the next request
    pub retry_after: Option<i64>,
    // The number of requests remaining in the current rate limit window
    pub ratelimit_remaining: Option<i64>,
    // The time at which the current rate limit window resets, in UTC epoch seconds
    pub ratelimit_reset: Option<i64>,
}

impl RateLimitErrorPayload {
    // When none of the rate limit headers is presents, there is not any rate limit error
    pub fn is_empty(&self) -> bool {
        return self.retry_after.is_none()
            && self.ratelimit_remaining.is_none()
            && self.ratelimit_reset.is_none();
    }

    // Returns the rate limit expiration time in seconds. If the function returns a value greater than 0,
    // that value should be considered as a limit of time in seconds to do the next request to the Github API
    // It applies the logic describe on the official Github API documentation:
    // https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api?apiVersion=2022-11-28#handle-rate-limit-errors-appropriately
    pub fn get_expiration_time(&self) -> i64 {
        // When retry_after contains a value, we should return it as expiration time. We do not need to do any conversion as Github API
        // already returns this value in seconds
        if let Some(retry_after) = self.retry_after {
            return retry_after;
        }

        let ratelimit_remaining = match self.ratelimit_remaining {
            Some(value) => value,
            None => i64::MAX,
        };

        // When ratelimit remaining is greater than 0, it means that we did not reach the rate limit amount of requests.
        if ratelimit_remaining > 0 {
            return 0;
        }

        let ratelimit_reset = match self.ratelimit_reset {
            Some(value) => value,
            None => 0,
        };

        if ratelimit_reset == 0 {
            return 0;
        }

        // We convert the rate limit reset from UTC epoch time to seconds.
        if let Some(reset_date) = DateTime::from_timestamp(ratelimit_reset, 0) {
            let today_date = Utc::now();
            let reset_expiration_date = today_date.signed_duration_since(reset_date);

            return reset_expiration_date.num_seconds();
        }

        // If we reach this code, it means that there is not any rate limit error
        0
    }

    pub fn from_response_headers(headers: &HeaderMap) -> Self {
        let mut retry_after: Option<i64> = None;
        let mut ratelimit_remaining: Option<i64> = None;
        let mut ratelimit_reset: Option<i64> = None;

        if let Some(value) = headers.get("retry-after") {
            let parsed_value = value.to_str().unwrap_or("");

            retry_after = match String::from(parsed_value).parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-remaining") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_remaining = match String::from(parsed_value).parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => None,
            };
        }

        if let Some(value) = headers.get("x-ratelimit-reset") {
            let parsed_value = value.to_str().unwrap_or("");

            ratelimit_reset = match String::from(parsed_value).parse::<i64>() {
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
