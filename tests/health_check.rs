use std::net::TcpListener;

#[tokio::test]
async fn health_check_success() {
    //Arrange
    let address = spawn_app();
    let client = reqwest::Client::new();

    //Act
    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to excute request");

    //Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    //Arrange
    let address = spawn_app();
    let client = reqwest::Client::new();
    let body = "name=John73&email=john_r77%40gmail.com";

    //Act
    let response = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to excute request");

    //Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    //Arrange
    let address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = [
        ("name=John73&email=", "email is missing"),
        ("name=&email=john_r77%40gmail.com", "name is missing"),
        ("name=&email=", "both email and name are missing"),
    ];

    //Act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to excute request");

        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with status code 400 when the payload was {}",
            error_message
        );
    }
}

fn spawn_app() -> String {
    //Retrieve random port assigned by the OS
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let server = zero2prod::run(listener).expect("Failed to bind address");

    let _ = tokio::spawn(server);

    // Return the address to the caller
    format!("http://127.0.0.1:{}", port)
}
