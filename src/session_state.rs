use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::FromRequest;
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";
    const PASSWROD_RESET_KEY: &'static str = "password_reset";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_password_reset(&self, is_needed: bool) -> Result<(), SessionInsertError> {
        self.0.insert(Self::PASSWROD_RESET_KEY, is_needed)
    }
    pub fn get_password_reset(&self) -> Result<Option<bool>, SessionGetError> {
        self.0.get(Self::PASSWROD_RESET_KEY)
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }
    pub fn get_user_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
