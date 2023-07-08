use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::{http::StatusCode, post, web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

// FormData impl's
impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(thiserror::Error)]
pub enum InsertDatabaseError {
    #[error("A database error was encountered while trying to store a subscription token.")]
    StoreTokenError(#[source] sqlx::Error),
    #[error("A database error was encountered while trying to insert a new subscriber.")]
    InsertSubscriberError(#[source] sqlx::Error),
}

// SubscribeError impl's
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

//InsertDatabaseError impl's
impl std::fmt::Debug for InsertDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[post("/subscriptions")]
#[tracing::instrument(
    name = "Adding a new subscriber.",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into()?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postges connection from the pool.")?;
    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscriber_token = generate_subscriber_token();

    store_token(subscriber_id, &subscriber_token, &mut transaction)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subsciber.")?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscriber_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &reqwest::Url,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let path = &format!(
        "/subscriptions/confirm?subscription_token={}",
        subscription_token
    );
    let confirmation_link = base_url
        .join(path)
        .expect("Invalid email confirmation path");

    let html_body = format!("Welcome to our newsletter!<br /> Click <a href=\"{}\">here</a> to confrim your subscription.", confirmation_link);
    let text_body = format!(
        "Welcome to our newsletter!\nPlease visit {} to confrim your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &text_body)
        .await
}

#[tracing::instrument(
    name = "Saving the new subscriber to the database.",
    skip(new_subscriber, transaction)
)]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, InsertDatabaseError> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO  subscriptions (id, email, name,subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        &new_subscriber.email.as_ref(),
        &new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(InsertDatabaseError::InsertSubscriberError)?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token to the database.",
    skip(subscription_token, transaction)
)]
async fn store_token(
    subscriber_id: Uuid,
    subscription_token: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), InsertDatabaseError> {
    sqlx::query!(
        r#"
        INSERT INTO  subscription_tokens (subscriber_id, subscription_token)
        VALUES ($1, $2)
        "#,
        subscriber_id,
        subscription_token
    )
    .execute(transaction)
    .await
    .map_err(InsertDatabaseError::StoreTokenError)?;
    Ok(())
}
fn generate_subscriber_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
