use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgConnection;
use std::net::TcpListener;

pub fn run(listener: TcpListener, connection: PgConnection) -> Result<Server, std::io::Error> {
    // Wrap the connection  in a smart pointer
    let connection = web::Data::new(connection);
    // Get a pointer copy and attach it to the application state
    let server = HttpServer::new(move || {
        App::new()
            .service(health_check)
            .service(subscribe)
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
