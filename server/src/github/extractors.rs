use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query},
    http::request::Parts,
    RequestPartsExt,
};

use crate::{
    errors::RustGoodFirstIssuesError, redis_utils::extractors::RedisKeyGeneratorExtractor,
};

use super::models::{
    GetGithubRepositoriesParams, GetGithubRepositoryGoodFirstIssuesParams,
    GetGithubRepositoryGoodFirstIssuesPathParams,
};

const DEFAULT_PER_PAGE: u32 = 10;
const DEFAULT_PAGE: u32 = 1;

#[derive(Debug)]
pub struct GithubRepositoriesKeyGenerator {
    pub params: GetGithubRepositoriesParams,
}

impl<S> RedisKeyGeneratorExtractor<S> for GithubRepositoriesKeyGenerator
where
    S: Send + Sync,
{
    fn generate_key(&self) -> String {
        format!(
            "github:repositories:rust:per_page={}&page={}",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE)
        )
    }
}
#[async_trait]
impl<S> FromRequestParts<S> for GithubRepositoriesKeyGenerator
where
    S: Send + Sync,
{
    type Rejection = RustGoodFirstIssuesError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let extracted_params = parts
            .extract::<Query<GetGithubRepositoriesParams>>()
            .await
            .unwrap();

        let params = GetGithubRepositoriesParams {
            page: extracted_params.page,
            per_page: extracted_params.per_page,
        };

        Ok(GithubRepositoriesKeyGenerator { params })
    }
}

#[derive(Debug)]
pub struct GithubGoodFirstIssuesKeyGenerator {
    pub path_params: GetGithubRepositoryGoodFirstIssuesPathParams,
    pub params: GetGithubRepositoryGoodFirstIssuesParams,
}

impl<S> RedisKeyGeneratorExtractor<S> for GithubGoodFirstIssuesKeyGenerator
where
    S: Send + Sync,
{
    fn generate_key(&self) -> String {
        format!(
            "github:issues:rust:per_page={}&page={}&owner={}&repository_name={}&labels=good_first_issue",
            self.params.per_page.unwrap_or(DEFAULT_PER_PAGE),
            self.params.page.unwrap_or(DEFAULT_PAGE),
            self.params.owner,
            self.path_params.repo
        )
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for GithubGoodFirstIssuesKeyGenerator
where
    S: Send + Sync,
{
    type Rejection = RustGoodFirstIssuesError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let extracted_params = parts
            .extract::<Query<GetGithubRepositoryGoodFirstIssuesParams>>()
            .await
            .unwrap();

        let extracted_path_params = parts
            .extract::<Path<GetGithubRepositoryGoodFirstIssuesPathParams>>()
            .await
            .unwrap();

        let params = GetGithubRepositoryGoodFirstIssuesParams {
            page: extracted_params.page,
            per_page: extracted_params.per_page,
            owner: extracted_params.owner.clone(),
        };
        let path_params = GetGithubRepositoryGoodFirstIssuesPathParams {
            repo: extracted_path_params.repo.clone(),
        };

        Ok(GithubGoodFirstIssuesKeyGenerator {
            params,
            path_params,
        })
    }
}
