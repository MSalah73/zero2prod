use actix_web::{post, web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use log;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[post("/subscriptions")]
async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    log::info!("request id: {} --- Adding {{ email: {} - name: {} }} as a new subscriber.",request_id, form.email, form.name);
    log::info!("request id: {} --- new the subscriber to the database.",request_id);
    match sqlx::query!(
        r#"
        INSERT INTO  subscriptions (id, email, name,subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => {

                log::info!("request id: {} --- New subscriber details have been saved in the database.", request_id);
                HttpResponse::Ok().finish()
            },
        Err(e) => {
            log::error!("request id: {} --- Failed to execute query: {:?}",request_id, e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
