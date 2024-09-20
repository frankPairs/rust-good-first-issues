use api::{
    app::App,
    config::{get_app_settings, Settings},
};
use axum::Router;
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::JsonAsyncCommands;
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub uuid: Uuid,
    pub settings: Settings,
    pub redis_pool: Pool<RedisConnectionManager>,
    pub github_server: MockServer,
    pub router: Router,
}

impl TestApp {
    pub async fn new() -> Self {
        let mut settings = get_app_settings().expect("Unable to get server settings");
        let github_server = MockServer::start().await;

        settings.application.set_port(0);
        settings.github.set_api_url(github_server.uri());

        let app = App::new(settings.clone()).await.unwrap();

        TestApp {
            settings,
            redis_pool: app.state.redis_pool.clone(),
            github_server,
            uuid: Uuid::new_v4(),
            router: app.router,
        }
    }

    pub async fn spawn_app(&self) -> String {
        let address = self
            .settings
            .application
            .get_addr()
            .expect("Unable to get http address for running the tests");

        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .expect("Unable to create a tcp listener");

        let base_url = listener.local_addr().unwrap();
        let router = self.router.clone();

        tokio::spawn(async move {
            axum::serve(listener, router)
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

    pub async fn redis_json_del(&self, key: String) {
        let mut redis_connection = self.redis_connection().await;

        let _: () = redis_connection.json_del(key, "$").await.unwrap();
    }
}
