use crate::authentication::{validate_credentials, AuthError, Credentials, Password};
use crate::routes::error_chain_fmt;
use crate::session_state::TypedSession;
use crate::utils::see_other;
use actix_web::error::InternalError;
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    skip(form, pool, session),
    fields(username=tracing::field::Empty, password=tracing::field::Empty)
)]
#[post("/login")]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password.clone(),
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            let password_feedback = Password::password_feedback(&form.0.password)
                .map_err(LoginError::UnexpectedError)
                .unwrap();
            let reset_needed = password_feedback.score < 3;

            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
            session
                .insert_password_reset(reset_needed)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;

            if reset_needed {
                return Ok(see_other("/admin/password"));
            }
            Ok(see_other("/admin/dashboard"))
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            FlashMessage::error(e.to_string()).send();
            let response = see_other("/login");
            Err(InternalError::from_response(e, response))
        }
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = see_other("/login");
    InternalError::from_response(e, response)
}
