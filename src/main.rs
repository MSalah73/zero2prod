use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, subscriber_init};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Setting up log env
    let subscriber = get_subscriber("zero2prod".into(), "trace".into(), std::io::stdout);
    subscriber_init(subscriber);

    // Panic if for some resson we can't read the configuration file
    let configuration = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
