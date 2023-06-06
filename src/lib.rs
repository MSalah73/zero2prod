use actix_web::dev::Server;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use serde;
use std::net::TcpListener;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}
#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/subscriptions")]
async fn subscribe(form: web::Form<FormData>) -> HttpResponse {
    if form.name.is_empty() || form.email.is_empty() {
        HttpResponse::BadRequest().finish()
    } else {
        HttpResponse::Ok().body(format!(
            "Form data: name: {} - email: {}",
            form.name, form.email
        ))
    }
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(health_check).service(subscribe))
        .listen(listener)?
        .run();
    Ok(server)
}
