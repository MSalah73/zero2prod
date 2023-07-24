use crate::utils::{e500, see_other};
use crate::{domain::SubscriberEmail, email_client::EmailClient};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
}

pub struct Newsletter {
    title: String,
    html_content: String,
    text_content: String,
}

impl Newsletter {
    fn parse(form: FormData) -> Result<Self, ()> {
        if form.title.trim().is_empty() {
            FlashMessage::error("Title is empty.").send();
            Err(())
        } else if form.html_content.trim().is_empty() {
            FlashMessage::error("Html content is empty.").send();
            Err(())
        } else if form.text_content.trim().is_empty() {
            FlashMessage::error("Plain text content is empty.").send();
            Err(())
        } else {
            Ok(Self {
                title: form.title,
                html_content: form.html_content,
                text_content: form.text_content,
            })
        }
    }
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish newsletter confirmed subscribers.",
    skip(form, email_client, pool)
)]
#[post("/newsletters")]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    email_client: web::Data<EmailClient>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let newsletter = match Newsletter::parse(form.0) {
        Err(_) => return Ok(see_other("/admin/newsletters")),
        Ok(newsletter) => newsletter,
    };

    let confirmed_subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &newsletter.title,
                    &newsletter.html_content,
                    &newsletter.text_content,
                )
                .await
                .with_context(|| format!("Failed to send newsletter issue to {}", subscriber.email))
                .map_err(e500)?,
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. \
                    Their stored contact detailed are invalid.")
            }
        }
    }
    FlashMessage::error("Newsletter issue has been published.").send();
    Ok(HttpResponse::Ok().finish())
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
