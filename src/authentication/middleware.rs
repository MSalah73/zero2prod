use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::InternalError;
use actix_web::{FromRequest, HttpMessage};
use actix_web_lab::middleware::Next;
use std::ops::Deref;
use uuid::Uuid;

//pub async fn reject_anonymous_users(session: &TypedSession) -> Result<Uuid, actix_web::Error> {
//    match session.get_user_id().map_err(e500)? {
//        Some(user_id) => Ok(user_id),
//        None => {
//            let response = see_other("/login");
//            let e = anyhow::anyhow!("The user has not logged in");
//            Err(InternalError::from_response(e, response).into())
//        },
//    }
//}

#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user_id().map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            next.call(req).await
        }
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}
// We can now use the session extactors as parameters
pub async fn force_password_change_on_weak_password(
    session: TypedSession,
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    if req.path() == "/admin/password" {
        return next.call(req).await;
    }

    match session.get_password_reset().map_err(e500)? {
        Some(true) => {
            let response = see_other("/admin/password");
            let e = anyhow::anyhow!(
                "User password does not meet password policy - forcing password reset"
            );
            Err(InternalError::from_response(e, response).into())
        }
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The reset flag has not been set");
            Err(InternalError::from_response(e, response).into())
        }
        _ => next.call(req).await,
    }
}
