//! lib.rs

use actix_web::dev::Server;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::net::TcpListener;

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}", &name)
}

async fn health_check() -> impl Responder {
    // OK returns a HttpResponseBuilder
    // HttpReponseBuilder implements Responsder
    // so it oka to omit the Fisish() method
    HttpResponse::Ok()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    // Why use TcpListner while we can do everything using HttpServer
    // Before HttpServer is doing two task; the first one is binding anddress
    // second is starting the application
    // With TcpListner, we can saparate the duties by binding the port
    // via TcpListner and then use HttpServer to listen and connect to it
    let server = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/health_check", web::get().to(health_check))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
