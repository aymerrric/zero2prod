use crate::{
    domain::SubscriberEmail,
    email_client::{ EmailClient},
    routes::subscriptions::error_chain_fmt,
};
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::{Context};
use serde::Deserialize;
use sqlx::{PgPool};

#[derive(Deserialize)]
pub struct Mail {
    pub title: String,
    pub content: MailContent,
}

#[derive(Deserialize)]
pub struct MailContent {
    pub html: String,
    pub text: String,
}

#[actix_web::post("/newsletter")]
pub async fn newsletter(
    connection: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    mail_to_send: web::Json<Mail>,
) -> Result<HttpResponse, NewsletterError> {
    let saved = get_confirmed_subscribers(&connection)
        .await
        .context("Failed to fetch the database")?;

    for mail in saved {
        match mail {
            Ok(email) => email_client
                .send_email(
                    &email.email,
                    &mail_to_send.title,
                    &mail_to_send.content.html,
                    &mail_to_send.content.text,
                )
                .await
                .with_context(|| format!("Could not send email to {}", email.email.as_ref()))?,
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                        Their stored contact details are invalid",
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
SELECT email
FROM subscriptions
WHERE status_subscription = 'confirmed'
"#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum NewsletterError {
    #[error(transparent)]
    CouldNotReadIntoTheDatabase(#[from] anyhow::Error),
}

impl std::fmt::Debug for NewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for NewsletterError {}

#[derive(sqlx::FromRow)]
pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
