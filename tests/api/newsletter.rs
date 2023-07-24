use crate::helper::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
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
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });

    let test_cases = vec![
        (
            serde_json::json!({
                "title": "",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"Newsletter body as plain text",
            }),
            r#"<p><i>Title is empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "          ",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"Newsletter body as plain text",
            }),
            r#"<p><i>Title is empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"",
                "text_content":"Newsletter body as plain text",
            }),
            r#"<p><i>Html content is empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"      ",
                "text_content":"Newsletter body as plain text",
            }),
            r#"<p><i>Html content is empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"",
            }),
            r#"<p><i>Plain text content is empty.</i></p>"#,
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "html_content":"<p>Newsletter body as HTML</P>",
                "text_content":"      ",
            }),
            r#"<p><i>Plain text content is empty.</i></p>"#,
        ),
    ];

    // Act 1 -- login
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act 2 -- issue newsletter
    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        // Assert
        assert_is_redirect_to(&response, "/admin/newsletters");

        // Act 3 -- follow redirect
        let html_page = app.get_newsletters_html().await;
        assert!(html_page.contains(error_message));
    }
}
#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });

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
    });

    // Act 1 -- login
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act 2 -- issue a newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 3 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body_request = serde_json::json!({
        "title": "Newsletter Title",
        "html_content":"<p>Newsletter body as HTML</P>",
        "text_content":"Newsletter body as plain text",
    });

    // Act 1 -- login
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act 2 -- issue a newsletter
    let response = app.post_newsletters(&newsletter_body_request).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 3 -- follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
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
