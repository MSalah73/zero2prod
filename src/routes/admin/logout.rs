use crate::session_state::TypedSession;
use crate::utils::see_other;
use actix_web::{post, HttpResponse};
use actix_web_flash_messages::FlashMessage;

#[post("/logout")]
pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.logout();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
