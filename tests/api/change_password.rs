use crate::helper::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn user_must_be_logged_in_to_access_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response = app.get_change_password().await;
    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn user_must_be_logged_in_to_change_the_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let change_password_body = serde_json::json!({
        "current_password" : &app.test_user.password,
        "new_password" : &new_password,
        "check_new_password" : &new_password,
    });

    // Act
    let response = app.post_change_password(&change_password_body).await;
    // Assert
    assert_is_redirect_to(&response, "/login")
}
#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;

    let login_body = serde_json::json!({
    "username": app.test_user.username,
    "password": app.test_user.password,
    });

    let new_password = Uuid::new_v4().to_string();
    let wrong_new_password = Uuid::new_v4().to_string();
    let change_password_body = serde_json::json!({
        "current_password" : &app.test_user.password,
        "new_password" : &new_password,
        "check_new_password" : &wrong_new_password,
    });

    // Act 1 -- Login and attempt to change password
    app.post_login(&login_body).await;

    let response = app.post_change_password(&change_password_body).await;

    // Assert 1
    assert_is_redirect_to(&response, "/admin/password");
    // Act 2 -- Follow the redirect
    let html_password_form = app.get_change_password_html().await;
    assert!(html_password_form.contains("<p><i>New password entries does not match.</i></p>"));
}
