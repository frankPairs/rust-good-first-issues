use reqwest::Client;

use crate::github::client::GithubHttpClient;
use crate::github::models::{
    GetGithubRepositoriesParams, GetGithubRepositoriesResponse,
    GetGithubRepositoryGoodFirstIssuesParams, GetGithubRepositoryGoodFirstIssuesPathParams,
    GetGithubRepositoryGoodFirstIssuesResponse, GithubIssue, GithubIssueAPI, GithubPullRequest,
    GithubRepository as GithubRepositoryModel, SearchGithubRepositoriesResponseAPI,
};
use crate::{config::GithubSettings, errors::RustGoodFirstIssuesError};

const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;

pub struct GithubRepositoriesHttpRepository {
    http_client: GithubHttpClient,
}

impl GithubRepositoriesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let http_client = GithubHttpClient::new(settings)?;

        Ok(Self { http_client })
    }

    #[tracing::instrument(name = "Get Rust repositories from Github API", skip(self))]
    pub async fn get(
        &self,
        params: &GetGithubRepositoriesParams,
    ) -> Result<GetGithubRepositoriesResponse, RustGoodFirstIssuesError> {
        let client: &Client = self.http_client.get_client();

        let mut url = self
            .http_client
            .get_base_url()?
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

        let response = client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(self.http_client.parse_error_from_response(response).await);
        }

        let json: SearchGithubRepositoriesResponseAPI = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetGithubRepositoriesResponse {
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
                    license: match repo.license {
                        Some(license) => Some(license.name),
                        None => None,
                    },
                })
                .collect(),
        })
    }
}

pub struct GithubGoodFirstIssuesHttpRepository {
    http_client: GithubHttpClient,
}

impl GithubGoodFirstIssuesHttpRepository {
    pub fn new(settings: GithubSettings) -> Result<Self, RustGoodFirstIssuesError> {
        let http_client = GithubHttpClient::new(settings)?;

        Ok(Self { http_client })
    }

    #[tracing::instrument(
        name = "Get Rust good first issues from a Github repository",
        skip(self)
    )]
    pub async fn get(
        &self,
        path_params: &GetGithubRepositoryGoodFirstIssuesPathParams,
        params: &GetGithubRepositoryGoodFirstIssuesParams,
    ) -> Result<GetGithubRepositoryGoodFirstIssuesResponse, RustGoodFirstIssuesError> {
        let client: &Client = self.http_client.get_client();
        let mut url = self
            .http_client
            .get_base_url()?
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

        let response = client
            .get(url)
            .send()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        if !response.status().is_success() {
            return Err(self.http_client.parse_error_from_response(response).await);
        }

        let json: Vec<GithubIssueAPI> = response
            .json()
            .await
            .map_err(RustGoodFirstIssuesError::ReqwestError)?;

        Ok(GetGithubRepositoryGoodFirstIssuesResponse {
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
