use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, subscriber_init};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Setting up log env
    let subscriber = get_subscriber("zero2prod".into(), "trace".into(), std::io::stdout);
    subscriber_init(subscriber);

    // Panic if for some resson we can't read the configuration file
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
