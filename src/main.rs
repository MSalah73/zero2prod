//! main.rs

use zero2prod::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ? - rise on Error
    run()?.await
}
