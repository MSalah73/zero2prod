//! tests/health_check.rs
//
// `actix_rt::test` is the testing equivalent ofd `actix_web::main`
//  Using `actix_rt` spare us from adding `#[test]` attribute
//
//
// To add `actix-rt` we have add it to cargo.toml via
// `cargo add actix-rt --dev --vers 2` then add it under the header
//  `[dev-dependencies]` in Cargo.toml
//
// To inspect what code gets generated, use `cargo expand --test health_check`
// where health_check is the name of the file

#[actix_rt::test]
async fn health_check_assessment() {
    spawn_app();

    // reqwest is black box testing tool -- interact with api with http request
    // reqwest decoupled from our actives this woild work if we change frameworks
    // like Ruby on Rails
    // Use`cargo add reqwest --dev --vers 0.11`to add
    // it under`[dev-dependencies]`in Cargo.toml
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() {
    let server = zero2prod::run().expect("Failed to bind address.");
    // Tokio runs the in the background as a background task
    // the spawn method takes a future and pass it ovet the
    // runtime for polling without waitinf for it compeletion which
    // it also runs concurrently with downstream feature and tasks
    // Does spawning leaves a running zombie process or does it gracefully kills
    // all associated tasks and processes when test program exits?
    // --- From Tokio::spawn docs
    // Spawning a task enables the task to execute concurrently to other tasks.
    // The spawned task may execute on the current thread, or it may be
    // sent to a different thread to be executed.
    // The specifics depend on the current Runtime configuration.
    //
    // There is no guarantee that a spawned task will execute
    //  to completion. When a runtime is shutdown, all outstanding
    // tasks are dropped, regardless of the lifecycle of that task.
    //
    // `cargo add tokio --dev --vers 1
    let _ = tokio::spawn(server);
}
