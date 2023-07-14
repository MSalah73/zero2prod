use actix_web::{post, web, HttpResponse};
use reqwest::header::LOCATION;
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct FormData {
    _username: String,
    _password: Secret<String>,
}

#[post("/login")]
pub async fn login(_form: web::Data<FormData>) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
