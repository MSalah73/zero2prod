use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, home, login,
    login_form, publish_newsletter, subscribe,
};
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

pub struct ApplicationBaseUrl(pub reqwest::Url);

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let app_base_url = configuration
            .application
            .base_url()
            .expect("Invalid application base url.");
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let url = configuration
            .email_client
            .url()
            .expect("Invalud email server url.");
        let timeout = configuration.email_client.tineout();
        let email_client = EmailClient::new(
            sender_email,
            url,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            app_base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: reqwest::Url,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    // Wrap the connection  in a smart pointer
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    // Secret key
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    // Flash messages middleware
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let messages_framework = FlashMessagesFramework::builder(message_store).build();

    // Redis middleware
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    // Get a pointer copy and attach it to the application state
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(messages_framework.clone())
            .service(health_check)
            .service(subscribe)
            .service(confirm)
            .service(publish_newsletter)
            .service(home)
            .service(login_form)
            .service(login)
            .service(admin_dashboard)
            .service(change_password_form)
            .service(change_password)
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
