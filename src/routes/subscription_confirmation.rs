use crate::routes::error_chain_fmt;
use actix_web::{get, http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(thiserror::Error)]
pub enum DatabaseError {
    #[error("A database error encountered while trying to fetch subscriber id from subsription_tokens table.")]
    GetSubscriberError(#[source] sqlx::Error),
    #[error(
        "A database error encountered while trying to update the subscriber's status to `confirmed`."
    )]
    ConfirmSubscriberError(#[source] sqlx::Error),
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            ConfirmationError::UnknownToken => StatusCode::UNAUTHORIZED,
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl std::fmt::Debug for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[get("/subscriptions/confirm")]
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let subscriber_id = get_subscriber_id_from_token(&parameters.subscription_token, &pool)
        .await
        .context("Failed to fetch subscriber id from the database with the provided token.")?
        .ok_or(ConfirmationError::UnknownToken)?;
    let _ = confirm_subscriber(subscriber_id, &pool)
        .await
        .context("Failed to update subscriber status to `confirmed`.");
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(subscriber_id: Uuid, pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(DatabaseError::ConfirmSubscriberError)?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
async fn get_subscriber_id_from_token(
    subscription_token: &str,
    pool: &PgPool,
) -> Result<Option<Uuid>, DatabaseError> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(DatabaseError::GetSubscriberError)?;

    Ok(result.map(|r| r.subscriber_id))
}
