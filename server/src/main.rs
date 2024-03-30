mod config;
mod errors;
mod github_repositories;
mod state;
mod telemetry;

use std::sync::Arc;

use anyhow::Error;
use axum::Router;
use config::get_app_settings;
use github_repositories::router::GithubRepositoryRouter;
use state::AppState;
use telemetry::{get_subscriber, init_subscriber};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let subscriber = get_subscriber(
        String::from("rust-good-first-issue-api"),
        String::from("info"),
    );

    // Initialize tracing subscriber
    init_subscriber(subscriber);

    let settings = get_app_settings().expect("Unable to get server settings");
    let addr = settings.application.get_addr()?;
    let github_settings = settings.github;

    let state = Arc::new(AppState { github_settings });

    let app = Router::new()
        .layer(CorsLayer::new().allow_origin(Any))
        .nest("/github_repositories", GithubRepositoryRouter::build())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Server running on {}", addr);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
