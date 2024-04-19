mod config;
mod errors;
mod github_repositories;
mod state;
mod telemetry;

use anyhow::Error;
use axum::Router;
use bb8_redis::RedisConnectionManager;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use config::get_app_settings;
use github_repositories::router::GithubRepositoryRouter;
use state::AppState;
use telemetry::{get_subscriber, init_subscriber};

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
    let redis_settings = settings.redis;

    let redis_manager = RedisConnectionManager::new(redis_settings.url).unwrap();
    let redis_pool = bb8::Pool::builder().build(redis_manager).await.unwrap();

    let state = Arc::new(AppState {
        github_settings,
        redis_pool,
    });

    let app = Router::new()
        .layer(CorsLayer::new().allow_origin(Any))
        .nest("/github_repositories", GithubRepositoryRouter::build())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Server running on {}", addr);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
