use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use std::net::TcpListener;
use zero2prod::configuration::{get_configuration, self, DatabaseSettings};
use zero2prod::startup::run;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool
}

#[tokio::test]
async fn health_check_success() {
    //Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    //Act
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to excute request");

    //Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    //Arrange
    let app = spawn_app().await;

    let client = reqwest::Client::new();
    let body = "name=John73&email=john_r77%40gmail.com";

    //Act
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to excute request");

    //Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "john_r77@gmail.com");
    assert_eq!(saved.name, "John73");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    //Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = [
        ("name=John73", "email is missing"),
        ("email=john_r77%40gmail.com", "name is missing"),
        ("", "both email and name are missing"),
    ];

    //Act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to excute request");

        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with status code 400 when the payload was {}",
            error_message
        );
    }
}

async fn spawn_app() -> TestApp {
    //Retrieve random port assigned by the OS
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;


    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");

    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db()).await.expect("Failed to connect to Postgres.");
    connection.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str()).await.expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string()).await.expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
    .run(&connection_pool)
        .await.expect("Failed to migrate the database.");
    connection_pool
}
