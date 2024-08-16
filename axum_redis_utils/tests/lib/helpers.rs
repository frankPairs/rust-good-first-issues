use axum::Router;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;

use crate::config::{get_app_settings, Settings};

struct TestApp {
    pub settings: Settings,
    pub redis_pool: Pool<RedisConnectionManager>,
}

impl TestApp {
    pub async fn run(app: Router) -> Self {
        let settings = get_app_settings().expect("Unable to get server settings");

        let application_settings = settings.application.clone();
        let redis_settings = settings.redis.clone();

        let redis_manager =
            RedisConnectionManager::new(redis_settings.url).expect("Redis manager failed");
        let redis_pool = bb8::Pool::builder()
            .build(redis_manager)
            .await
            .expect("Redis pool connection failed");

        let address = application_settings
            .get_addr()
            .expect("Unable to get http address for running the tests");
        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .expect("Unable to create a tcp listener");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Error running the test server");
        });

        TestApp {
            redis_pool,
            settings,
        }
    }
}
