use sqlx::PgPool;
use std::net::TcpListener;
use secrecy::ExposeSecret;
use zero2prod_my::configuration::get_configuration;
use zero2prod_my::startup::run;
use zero2prod_my::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    let configuration = get_configuration().expect("Failed to read configuration");
    let configuration_string = configuration.database.connection_string();
    let connection_pool = PgPool::connect(&configuration_string.expose_secret())
        .await
        .expect("Failed to connect Postgres.");
    run(listener, connection_pool)?.await
}
