use axum::Router;
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis_macros::FromRedisValue;
use serde::{Deserialize, Serialize};

use crate::config::{get_app_settings, Settings};

pub struct TestApp {
    pub settings: Settings,
    pub redis_pool: Pool<RedisConnectionManager>,
}

impl TestApp {
    pub async fn new() -> Self {
        let settings = get_app_settings().expect("Unable to get server settings");
        let redis_settings = settings.redis.clone();
        let redis_manager =
            RedisConnectionManager::new(redis_settings.url).expect("Redis manager failed");
        let redis_pool = bb8::Pool::builder()
            .build(redis_manager)
            .await
            .expect("Redis pool connection failed");

        TestApp {
            redis_pool,
            settings,
        }
    }

    pub async fn spawn_app(&self, app: Router) -> String {
        let address = self
            .settings
            .application
            .get_addr()
            .expect("Unable to get http address for running the tests");

        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .expect("Unable to create a tcp listener");

        let base_url = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Error running the test server");
        });

        format!("http://{}", base_url)
    }

    pub async fn redis_connection(&self) -> PooledConnection<RedisConnectionManager> {
        self.redis_pool
            .get()
            .await
            .expect("Unable to get redis connection")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRedisValue)]
pub struct TestHandlerResponse {
    pub status: i64,
    pub message: String,
}
