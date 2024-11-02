use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use redis_macros::FromRedisValue;
use serde::{Deserialize, Serialize};

const DEFAULT_RATE_LIMIT_EXP: i64 = 600;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, FromRedisValue)]
pub struct GithubRateLimitError {
    // The time in seconds that you should wait before making the next request
    pub retry_after: Option<i64>,
    // The number of requests remaining in the current rate limit window
    pub ratelimit_remaining: Option<i64>,
    // The time at which the current rate limit window resets, in UTC epoch seconds
    pub ratelimit_reset: Option<i64>,
}

impl GithubRateLimitError {
    // Returns the rate limit expiration time in seconds. If the function returns a value greater than 0,
    // that value should be considered as a limit of time in seconds to do the next request to the Github API
    //
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

        let ratelimit_reset = self.ratelimit_reset.unwrap_or(0);
        if ratelimit_reset == 0 {
            return 0;
        }

        // We convert the rate limit reset from UTC epoch time to seconds.
        if let Some(reset_date) = DateTime::from_timestamp(ratelimit_reset, 0) {
            let today_date = Utc::now();
            let reset_expiration_date = reset_date.signed_duration_since(today_date);

            return reset_expiration_date.num_seconds();
        }

        DEFAULT_RATE_LIMIT_EXP
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

        GithubRateLimitError {
            ratelimit_remaining,
            ratelimit_reset,
            retry_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn test_get_expiration_time_when_retry_after_is_present() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: Some(10),
            ratelimit_remaining: None,
            ratelimit_reset: None,
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 10);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_remaining_is_greater_than_zero() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: Some(10),
            ratelimit_reset: None,
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 0);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_reset_is_zero() {
        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: None,
            ratelimit_reset: Some(0),
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 0);
    }

    #[test]
    fn test_get_expiration_time_when_ratelimit_remaining_is_zero_and_ratelimit_reset_is_greater_than_zero(
    ) {
        let tomorrow = Utc::now() + Duration::days(1);

        let rate_limit_error = GithubRateLimitError {
            retry_after: None,
            ratelimit_remaining: Some(0),
            ratelimit_reset: Some(tomorrow.timestamp()),
        };

        assert_eq!(rate_limit_error.get_expiration_time(), 86399);
    }
}
