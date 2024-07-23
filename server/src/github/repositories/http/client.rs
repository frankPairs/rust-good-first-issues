use axum::http::HeaderMap;
use reqwest::{header, Client, Url};

use crate::errors::{RateLimitErrorPayload, RustGoodFirstIssuesError};

const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_API_USERNAME: &str = "frankPairs";

#[derive(Debug, serde::Deserialize)]
struct GithubApiErrorPayload {
    message: String,
}

pub struct GithubHttpClient {
    client: Client,
}

impl GithubHttpClient {
    pub fn new(github_token: String) -> Result<Self, RustGoodFirstIssuesError> {
        let mut headers = header::HeaderMap::new();

        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", github_token).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", GITHUB_API_VERSION.parse().unwrap());
        headers.insert("User-Agent", GITHUB_API_USERNAME.parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(Self { client })
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_base_url(&self) -> Result<Url, RustGoodFirstIssuesError> {
        Url::parse(GITHUB_API_BASE_URL).map_err(RustGoodFirstIssuesError::ParseUrlError)
    }

    pub async fn try_into_error(&self, response: reqwest::Response) -> RustGoodFirstIssuesError {
        let status_code = response.status();
        let headers = response.headers().clone();
        let result: Result<GithubApiErrorPayload, reqwest::Error> = response.json().await;

        match result {
            Ok(error_payload) => {
                let err_message = error_payload.message;
                let rate_limit_err_payload = RateLimitErrorPayload::from_response_headers(&headers);

                if !rate_limit_err_payload.is_empty() {
                    return RustGoodFirstIssuesError::GithubRateLimitError(
                        err_message,
                        rate_limit_err_payload,
                    );
                }

                RustGoodFirstIssuesError::GithubAPIError(status_code, err_message)
            }
            Err(err) => RustGoodFirstIssuesError::ReqwestError(err),
        }
    }
}
