use crate::helper::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use std::time::Duration;
use wiremock::matchers::{any, method, path};
use wiremock::MockBuilder;
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn user_must_be_logged_in_access_newsletters_form() {
    // Arrange
    let app = spawn_app().await;

    // Act 1 -- attempt to access newsletters form
    let response = app.get_newsletters().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn user_must_be_logged_in_to_publish_a_newsletter() {
    // Arrange
    let app = spawn_app().await;
    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    // Act 1 -- attempt to issue newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn newsletters_form_send_appropriate_message_on_empty_fields() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (
            serde_json::json!({
                "title": "",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Title cannot be empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "          ",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Title cannot be empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"",
                "text_content":"Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Html content cannot be empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"      ",
                "text_content":"Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Html content cannot be empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Plain text content cannot be empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"      ",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            r#"<p><i>Plain text content cannot be empty.</i></p>"#,
        ),
    ];

    // Act 1 -- issue newsletter
    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        // Assert
        assert_is_redirect_to(&response, "/admin/newsletters");

        // Act 2 -- follow redirect
        let html_page = app.get_newsletters_html().await;
        assert!(html_page.contains(error_message));
    }
}
#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    let _ = create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;
    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    // Act 1 -- issue a newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // mock the background worker with speculating if the background process is fisnied
    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    // Act 1 -- issue a newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    app.dispatch_all_pending_emails().await;
}
#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    // Act 1 -- issue a newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    // Act 3 -- issue a newsletter again
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 4 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    app.dispatch_all_pending_emails().await;

    //Mock verifes on Drop that we have sent the newsletter email --ONCE--
}
#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefulliy() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        // We need a long enough delay to ensure that the
        // second request arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    // Act -- Sending a retry right after the first request
    let response1 = app.post_newsletters(&newsletter_body_request);
    let response2 = app.post_newsletters(&newsletter_body_request);
    let (response1, response2) = tokio::join!(response1, response2);

    // Assert
    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    app.dispatch_all_pending_emails().await;

    //Mock verifes on Drop that we have sent the newsletter email --ONCE--
}
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    // We need to use _veriable_name for the guard to be droped at the end of the scope
    // We can not use _
    let _mock_guard = when_sending_an_email()
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
// short hand for a commaon mocking setup
fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}
