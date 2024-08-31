use axum::Router;

use api::{
    app::AppBuilder,
    config::{get_app_settings, Settings},
};

pub struct TestApp {
    pub settings: Settings,
    pub app: Router,
}

impl TestApp {
    pub async fn new() -> Self {
        let settings = get_app_settings().expect("Unable to get server settings");
        let app = AppBuilder::new(settings.clone()).build().await.unwrap();

        TestApp { settings, app }
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

        let app = self.app.clone();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Error running the test server");
        });

        format!("http://{}", base_url)
    }
}
