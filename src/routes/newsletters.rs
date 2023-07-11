use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};
use actix_web::{
    http::{
        header,
        header::{HeaderMap, HeaderValue},
        StatusCode,
    },
    post, web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

pub struct Credentials {
    username: String,
    password: Secret<String>,
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PublishNewsletterError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishNewsletterError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishNewsletterError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

impl std::fmt::Debug for PublishNewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name = "Publish newsletter confirmed subscribers.",
    skip(body, email_client, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
#[post("/newsletters")]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PublishNewsletterError> {
    let credentials =
        basic_authentication(request.headers()).map_err(PublishNewsletterError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    tracing::Span::current().record(
        "user_id",
        &tracing::field::display(&credentials.password.expose_secret()),
    );
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    let confirmed_subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to fetch confirmed subscriber from the database.")?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter issue to {}", subscriber.email)
                })?,
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. \
                    Their stored contact detailed are invalid.")
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' was missing.")?
        .to_str()
        .context("The 'Authorization' header value was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'baisc'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to decode base64-decode 'basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(
    name = "Validate auth credentials against the database.",
    skip(credentials, pool)
)]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishNewsletterError> {
    let user_id: Option<_> = sqlx::query!(
        r#"
        SELECT user_id
        FROM users
        WHERE username = $1 AND password = $2
        "#,
        credentials.username,
        credentials.password.expose_secret()
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")
    .map_err(PublishNewsletterError::UnexpectedError)?;

    user_id
        .map(|row| row.user_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password."))
        .map_err(PublishNewsletterError::AuthError)
}

#[tracing::instrument(name = "Fetch all confirmed subscribers.", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    //.expect("A database error encountered while trying to fetch all confirmed subscribers.");
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(rows)
}
