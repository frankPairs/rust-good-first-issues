use redis_macros::FromRedisValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchGithubRepositoriesResponseAPI {
    pub total_count: u32,
    pub items: Vec<GithubRepositoryAPI>,
}

#[derive(Debug, Deserialize)]
pub struct GithubRepositoryAPI {
    pub id: u32,
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub stargazers_count: u32,
    pub open_issues_count: u32,
    pub has_issues: bool,
    pub owner: GithubRepositoryOwnerAPI,
    pub license: Option<GithubRepositoryLicenseAPI>,
}

#[derive(Debug, Deserialize)]
pub struct GithubIssueAPI {
    pub id: u32,
    pub title: String,
    pub description: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    pub state: GithubIssueState,
    pub pull_request: Option<GithubPullRequestAPI>,
}

#[derive(Debug, Deserialize)]
pub struct GithubPullRequestAPI {
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubRepositoryOwnerAPI {
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubRepositoryLicenseAPI {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GithubRepository {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub private: bool,
    pub avatar_url: String,
    pub description: Option<String>,
    pub stars_count: u32,
    pub open_issues_count: u32,
    pub has_issues: bool,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubIssue {
    pub id: u32,
    pub title: String,
    pub description: Option<String>,
    pub body: Option<String>,
    pub url: String,
    pub state: GithubIssueState,
    pub pull_request: Option<GithubPullRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubPullRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GithubIssueState {
    Open,
    Close,
}

#[derive(Debug, Deserialize)]
pub struct GetGithubRepositoriesParams {
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRedisValue)]
pub struct GetGithubRepositoriesResponse {
    pub total_count: u32,
    pub items: Vec<GithubRepository>,
}

#[derive(Debug, Deserialize)]
pub struct GetGithubRepositoryGoodFirstIssuesParams {
    pub owner: String,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GetGithubRepositoryGoodFirstIssuesPathParams {
    pub repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRedisValue)]
pub struct GetGithubRepositoryGoodFirstIssuesResponse {
    pub items: Vec<GithubIssue>,
}
