use crate::helper::assert_is_redirect_to;
use crate::helper::spawn_app;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;
    let login_body = serde_json::json!({
    "username":"random-username",
    "password":"random-password",
    });

    // Act 1 --  Try to login
    let response = app.post_login(&login_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");

    // Act 2 -- Follow the redirect
    let html_page = app.get_login_html().await;

    // Assert
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Acr 3 -- Reload the login page
    let html_page = app.get_login_html().await;

    // Assert
    assert!(!html_page.contains(r#"Authentication failed"#));
}
