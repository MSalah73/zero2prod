use crate::helper::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    //Arrange
    let app = spawn_app().await;

    let body = "name=John73&email=john_r77%40gmail.com";

    //Act
    let response = app.post_subscriptions(body.into()).await;

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
