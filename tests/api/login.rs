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

    // Act
    let response = app.post_login(&login_body).await;

    let flash_cookies = response.cookies().find(|c| c.name() == "_flash").unwrap();

    // Assert
    assert_is_redirect_to(&response, "/login");
    assert_eq!(flash_cookies.value(), "Authentication failed")
}
