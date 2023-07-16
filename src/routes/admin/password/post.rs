use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    _current_password: Secret<String>,
    new_password: Secret<String>,
    check_new_password: Secret<String>,
}

#[tracing::instrument(name = "Change password", skip(form, session, _pool))]
#[post("/admin/password")]
pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    _pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }

    if form.new_password.expose_secret() != form.check_new_password.expose_secret() {
        FlashMessage::error("New password entries does not match.").send();
        return Ok(see_other("/admin/password"));
    }
    Ok(HttpResponse::Ok().finish())
}
