use crate::authentication::Password;
use crate::authentication::UserId;
use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::admin::dashboard::get_username;
use crate::utils::{e500, see_other};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    check_new_password: Secret<String>,
}

#[tracing::instrument(name = "Change password", skip(form, pool))]
#[post("/password")]
pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    if form.new_password.expose_secret() != form.check_new_password.expose_secret() {
        FlashMessage::error("New password entries does not match.").send();
        return Ok(see_other("/admin/password"));
    }

    let new_passeord = match Password::parse(&form.new_password) {
        Ok(password) => password,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return Ok(see_other("/admin/password"));
        }
    };

    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.current_password.clone(),
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(*user_id, new_passeord, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
