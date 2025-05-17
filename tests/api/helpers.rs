use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod_my::configuration::{get_configuration, DatabaseSettings};
use zero2prod_my::startup::{get_connection_pool, Application};
use zero2prod_my::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_lever = "info".into();
    let subscriber_name = "zero2prod".into();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_lever, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_lever, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");
        response
    }

/* <<<<<<<<<<<<<<  ✨ Windsurf Command ⭐ >>>>>>>>>>>>>>>> */
    /// Extract the confirmation links embedded in the request to the email server
    ///
    /// We use `linkify` to extract the links from the HTML and text bodies of the
    /// email request. We assert that there is exactly one link in each body, and
    /// return the two links as a `ConfirmationLinks` struct.
    ///
    /// The links are modified to include the port that the test app is running on,
    /// which is not known when the links are generated.
/* <<<<<<<<<<  b07b3fa8-3587-4abd-b57b-b4cb235e1e33  >>>>>>>>>>> */
    pub async fn get_confirmation_links(&self) -> ConfirmationLinks {
        let request = &self.email_server.received_requests().await.unwrap()[0];
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            links[0].as_str().to_owned()
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let text = get_link(&body["TextBody"].as_str().unwrap());
        let mut html = Url::parse(&html).unwrap();
        let mut text = Url::parse(&text).unwrap();
        html.set_port(Some(self.port)).unwrap();
        text.set_port(Some(self.port)).unwrap();
        ConfirmationLinks { html, text }
    }
}


pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;

    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration.application.port = 0;
    configuration.email_client.base_url = email_server.uri();
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone()).await.expect("Failed to build application");
    let port = application.port();
    let address = format!("http://localhost:{}", port);
    let _ = tokio::spawn(application.run_until_stopped());
    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection =
        PgConnection::connect_with(&config.without_db())
            .await
            .expect("Failed to connect Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}



