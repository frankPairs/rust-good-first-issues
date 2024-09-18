use api::{
    app::App,
    config::{get_app_settings, Settings},
};
use redis::JsonAsyncCommands;
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub settings: Settings,
    pub app: App,
    pub github_server: MockServer,
    pub uuid: Uuid,
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
            app,
            github_server,
            uuid: Uuid::new_v4(),
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
        let router = self.app.router.clone();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("Error running the test server");
        });

        format!("http://{}", base_url)
    }

    pub async fn redis_json_del(&self, key: String) {
        let redis_pool = self.app.state.redis_pool.clone();
        let mut redis_connection = redis_pool.get().await.unwrap();

        let _: () = redis_connection.json_del(key, "$").await.unwrap();
    }
}
