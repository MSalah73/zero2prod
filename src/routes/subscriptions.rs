use actix_web::{post, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
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
