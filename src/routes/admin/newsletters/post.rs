use crate::authentication::UserId;
use crate::email_client::EmailClient;
use crate::idempotency::{save_response, try_processing, IdempotencyKey, NextAction};
use crate::issue_delivery_worker::Newsletter;
use crate::utils::{e400, e500, see_other};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

// Genereally we want both empty field to return bad request but because we want to redirect
// the user to the form again we need to 303 instead of 400.
impl TryFrom<FormData> for Newsletter {
    type Error = anyhow::Error;
    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let FormData {
            title,
            html_content,
            text_content,
            ..
        } = form;
        if title.trim().is_empty() {
            FlashMessage::error("Title cannot be empty.").send();
            anyhow::bail!("Title is empty")
        } else if html_content.trim().is_empty() {
            FlashMessage::error("Html content cannot be empty.").send();
            anyhow::bail!("Html content is empty")
        } else if text_content.trim().is_empty() {
            FlashMessage::error("Plain text content cannot be empty.").send();
            anyhow::bail!("Plain text content is empty")
        } else {
            Ok(Self {
                title,
                html_content,
                text_content,
            })
        }
    }
}

impl From<Newsletter> for FormData {
    fn from(nl: Newsletter) -> Self {
        nl.into()
    }
}

#[tracing::instrument(
    name = "Publish newsletter confirmed subscribers.",
    skip(form, _email_client, pool)
)]
#[post("/newsletters")]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    user_id: web::ReqData<UserId>,
    _email_client: web::Data<EmailClient>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let idempotency_key: IdempotencyKey =
        form.idempotency_key.to_owned().try_into().map_err(e400)?;
    let user_id = user_id.into_inner();

    let newsletter: Newsletter = match form.0.try_into() {
        Err(_) => return Ok(see_other("/admin/newsletters")),
        Ok(newsletter) => newsletter,
    };

    // Return early if we have a saved response in the database
    let mut transaction = match try_processing(*user_id, &idempotency_key, &pool)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };
    let issue_id = insert_newsletter_issue(&mut transaction, &newsletter)
        .await
        .context("Failed to store newsletter issue details.")
        .map_err(e500)?;
    enqueue_delivery_task(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks.")
        .map_err(e500)?;
    let response = see_other("/admin/newsletters");
    // save response dissect the response store it in the database then reassemble it into a new
    // response
    let response = save_response(*user_id, &idempotency_key, response, transaction)
        .await
        .map_err(e500)?;
    success_message().send();
    Ok(response)
}

#[tracing::instrument(name = "Adding newsletter to database.", skip(transaction, newsletter))]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter: &Newsletter,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            html_content,
            text_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        newsletter.title,
        newsletter.html_content,
        newsletter.text_content
    )
    .execute(transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(name = "Enqueuing delivery task in the database.", skip(transaction))]
async fn enqueue_delivery_task(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email 
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
}
