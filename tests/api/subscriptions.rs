use crate::helper::spawn_app;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    // Act
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    //Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    //Act
    let response = app.post_subscriptions(body.into()).await;

    //Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_presists_the_new_subscriber() {
    //Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    //Act
    app.post_subscriptions(body.into()).await;

    //Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "john_r77@gmail.com");
    assert_eq!(saved.name, "John73");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    //Arrange
    let app = spawn_app().await;

    let test_cases = [
        ("name=John73", "email is missing"),
        ("email=john_r77%40gmail.com", "name is missing"),
        ("", "both email and name are missing"),
    ];

    //Act
    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with status code 400 when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_invalid() {
    //Arrange
    let app = spawn_app().await;

    let test_cases = [
        ("name=&email=john_r77%40gmail.com", "name is empty"),
        (
            "name={John}&email=john_r77%40gmail.com",
            "name contain forbidden characters",
        ),
        (
            "name=John/&email=john_r77%40gmail.com",
            "name contain forbidden characters",
        ),
        (
            "name=John's&email=john_r77%40gmail.com",
            "name contain forbidden characters",
        ),
        (
            "name=John\"s&email=john_r77%40gmail.com",
            "name contain forbidden characters",
        ),
        ("name=John73&email=definitely-not-an-email", "invalid email"),
    ];

    //Act
    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with status code 400 when the payload was {}",
            error_message
        );
    }
}
