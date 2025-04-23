use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod_my::configuration::get_configuration;
use zero2prod_my::email_client::EmailClient;
use zero2prod_my::startup::run;
use zero2prod_my::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");

    let address = format!(
        "{}:{}",
        &configuration.application.host, &configuration.application.port
    );
    let sender_email = configuration.email_client.sender().expect("123");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(configuration.email_client.base_url, sender_email,
                                        configuration.email_client.authorization_token, timeout,
    );
    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    let configuration = get_configuration().expect("Failed to read configuration");
    let configuration_options = configuration.database.with_db();
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration_options);
    run(listener, connection_pool, email_client)?.await
}
