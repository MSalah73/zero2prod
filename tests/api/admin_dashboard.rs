use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn user_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response = app.get_admin_dashboard().await;
    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn logout_clears_session_state() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    // Act 1 -- Follow redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Act 2 -- Logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Act 3 -- Follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act 4 -- attempt to navigate to admin panel
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
