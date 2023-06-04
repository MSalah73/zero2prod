use actix_web::{get, App, HttpServer, HttpResponse, Responder};

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

#[tokio::main]
async fn main() -> Result<(),std::io::Error> {
    HttpServer::new(|| {
        App::new()
            .service(health_check)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
