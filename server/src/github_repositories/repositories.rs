use reqwest::{header, Client, Url};

use super::models::{
    GetRustRepositoriesParams, GetRustRepositoriesResponse, GetRustRepositoryGoodFirstIssuesParams,
    GetRustRepositoryGoodFirstIssuesPathParams, GetRustRepositoryGoodFirstIssuesResponse,
    GithubIssue, GithubIssueAPI, GithubPullRequest, SearchGithubRepositoriesResponseAPI,
};
use crate::github_repositories::models::GithubRepository as GithubRepositoryModel;
use crate::{config::GithubSettings, errors::RustGoodFirstIssuesError};

const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;

#[derive(Debug)]
pub struct GithubRepository {
    client: Client,
}

impl GithubRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let github_token = settings.get_token();
        let mut headers = header::HeaderMap::new();

        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", github_token).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", GITHUB_API_VERSION.parse().unwrap());
        headers.insert("User-Agent", "frankPairs".parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(Self { client })
    }

    pub async fn get_rust_repositories(
        &self,
        params: GetRustRepositoriesParams,
    ) -> Result<GetRustRepositoriesResponse, RustGoodFirstIssuesError> {
        let mut url = Url::parse(GITHUB_API_BASE_URL)
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?
            .join("/search/repositories?")
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?;

        url.query_pairs_mut()
            .append_pair("q", "language:rust")
            .append_pair("sort", "help-wanted-issues")
            .append_pair("order", "desc")
            .append_pair(
                "per_page",
                &params.per_page.unwrap_or(DEFAULT_PER_PAGE).to_string(),
            )
            .append_pair("page", &params.page.unwrap_or(DEFAULT_PAGE).to_string());

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(RustGoodFirstIssuesError::GithubAPIError(
                response.status(),
                "Github API error while fetching repositories".to_string(),
            ));
        }

        let json: SearchGithubRepositoriesResponseAPI = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetRustRepositoriesResponse {
            total_count: json.total_count,
            items: json
                .items
                .into_iter()
                .map(|repo| GithubRepositoryModel {
                    id: repo.id,
                    url: repo.html_url,
                    name: repo.full_name,
                    private: repo.private,
                    avatar_url: repo.owner.avatar_url,
                    description: repo.description,
                    stars_count: repo.stargazers_count,
                    open_issues_count: repo.open_issues_count,
                    has_issues: repo.has_issues,
                    license: repo.license.name,
                })
                .collect(),
        })
    }

    pub async fn get_repository_issues(
        &self,
        path_params: GetRustRepositoryGoodFirstIssuesPathParams,
        params: GetRustRepositoryGoodFirstIssuesParams,
    ) -> Result<GetRustRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let mut url = Url::parse(GITHUB_API_BASE_URL)
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?
            .join(&format!(
                "/repos/{}/{}/issues?",
                params.owner, path_params.repo
            ))
            .map_err(RustGoodFirstIssuesError::ParseUrlError)?;

        url.query_pairs_mut()
            .append_pair("labels", "good first issue")
            .append_pair("sort", "updated")
            .append_pair("direction", "desc")
            .append_pair(
                "per_page",
                &params.per_page.unwrap_or(DEFAULT_PER_PAGE).to_string(),
            )
            .append_pair("page", &params.page.unwrap_or(DEFAULT_PAGE).to_string());

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(RustGoodFirstIssuesError::GithubAPIError(
                response.status(),
                "Github API error while fetching issues".to_string(),
            ));
        }

        let json: Vec<GithubIssueAPI> = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetRustRepositoryGoodFirstIssuesResponse {
            items: json
                .into_iter()
                .map(|issue| GithubIssue {
                    id: issue.id,
                    body: issue.body,
                    description: issue.description,
                    state: issue.state,
                    title: issue.title,
                    url: issue.html_url,
                    pull_request: if let Some(pull_request) = issue.pull_request {
                        Some(GithubPullRequest {
                            url: pull_request.html_url,
                        })
                    } else {
                        None
                    },
                })
                .collect(),
        })
    }
}
