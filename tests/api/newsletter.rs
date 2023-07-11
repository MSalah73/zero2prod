use crate::helper::{spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    // Arrange
    let app = spawn_app().await;

    let body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "html":"<p>Newsletter body as HTML</P>",
            "text":"Newsletter body as plain text",
        },
    });

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}
#[tokio::test]
async fn newsletterr_return_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "html":"<p>Newsletter body as HTML</P>",
                    "text":"Newsletter body as plain text",
                },
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title"
            }),
            "missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "content": {
                    "text":"Newsletter body as plain text",
                },
            }),
            "missing html",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "content": {
                    "html":"<p>Newsletter body as HTML</P>",
                },
            }),
            "missing text",
        ),
        (serde_json::json!({}), "empty"),
    ];

    // Act
    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with a 400 `Bad Request` when payload was {}",
            error_message
        );
    }
}
#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;

    let _ = create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "html":"<p>Newsletter body as HTML</P>",
            "text":"Newsletter body as plain text",
        },
    });

    // Act
    let response = app.post_newsletters(&newsletter_body_request).await;
    // Asserrt
    assert_eq!(response.status().as_u16(), 200);
    //Mock verifies on Drop that we haven't send any newsletters
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "html":"<p>Newsletter body as HTML</P>",
            "text":"Newsletter body as plain text",
        },
    });

    // Act
    let response = app.post_newsletters(&newsletter_body_request).await;

    // Asserrt
    assert_eq!(response.status().as_u16(), 200);
}
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=John73&email=john_r77%40gmail.com";

    // We need to use _veriable_name for the guard to be droped at the end of the scope
    // We can not use _
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
