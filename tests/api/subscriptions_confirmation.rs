use crate::helper::spawn_app;
use reqwest;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirm_a_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=John73&email=john_r77%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // Act
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert

    let saved = sqlx::query!("SELECT email,name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "john_r77@gmail.com");
    assert_eq!(saved.name, "John73");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn the_link_returned_by_the_subscribe_returns_a_200_when_called() {
    // Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    // Act
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();
    // Assert
    assert_eq!(response.status().as_u16(), 400);
}
