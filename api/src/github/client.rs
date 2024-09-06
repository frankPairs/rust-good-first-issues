use reqwest::{header, Client, Url};

use crate::{config::GithubSettings, errors::RustGoodFirstIssuesError};

const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_API_USERNAME: &str = "frankPairs";

#[derive(Debug, serde::Deserialize)]
struct GithubApiErrorPayload {
    message: String,
}

pub struct GithubHttpClient {
    client: Client,
    settings: GithubSettings,
}

impl GithubHttpClient {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let mut headers = header::HeaderMap::new();

        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", settings.get_token()).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", GITHUB_API_VERSION.parse().unwrap());
        headers.insert("User-Agent", GITHUB_API_USERNAME.parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(Self { client, settings })
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_base_url(&self) -> Result<Url, RustGoodFirstIssuesError> {
        Url::parse(&self.settings.get_api_url()).map_err(RustGoodFirstIssuesError::ParseUrlError)
    }

    pub async fn parse_error_from_response(
        &self,
        response: reqwest::Response,
    ) -> RustGoodFirstIssuesError {
        let status_code = response.status();
        let headers = response.headers().clone();
        let result: Result<GithubApiErrorPayload, reqwest::Error> = response.json().await;

        match result {
            Ok(error_payload) => RustGoodFirstIssuesError::GithubAPIError(
                status_code,
                headers,
                error_payload.message,
            ),
            Err(err) => RustGoodFirstIssuesError::ReqwestError(err),
        }
    }
}
