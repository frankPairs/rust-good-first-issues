use crate::config::GithubSettings;

#[derive(Clone, Debug)]
pub struct AppState {
    pub github_settings: GithubSettings,
}
