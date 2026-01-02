use std::fmt::Display;

use crate::domain::{NewSubscriber, SubscriberEmail, SuscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::{HttpResponse, post, web};
use anyhow::Context;
use chrono::Utc;
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use serde::Deserialize;
use sqlx::PgPool;
use tracing;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionForm {
    email: String,
    name: String,
}
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, connection, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subcriber_name = %form.name
)
)]
#[post("/subscription")]
async fn subscribe(
    form: web::Form<SubscriptionForm>,
    connection: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber =
        form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = connection
        .begin()
        .await
        .context("Failed to acquire a connection from the pool")?;
    let subscriber_id = insert_suscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert the user into database")?;
    let token = generate_random_token();
    store_token(&mut transaction, subscriber_id, &token)
        .await
        .context("Failed to store the token into the database")?;
    transaction
        .commit()
        .await
        .context("Failed to commit the transaction into the database")?;
    send_email(&email_client, new_subscriber, base_url, &token)
        .await
        .context("Failed to send confirmation email")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Start subscription querry", skip(transaction, newsubscriber))]
pub async fn insert_suscriber(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    newsubscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status_subscription)
                    VALUES($1, $2, $3, $4, 'pending_confirmation')"#,
        id,
        newsubscriber.email.as_ref(),
        newsubscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Could not subscribe bacause {}", e);
        e
    })?;
    Ok(id)
}

#[tracing::instrument(name = "Send an email", skip(email_client, newsubscriber, base_url))]
pub async fn send_email(
    email_client: &EmailClient,
    newsubscriber: NewSubscriber,
    base_url: web::Data<ApplicationBaseUrl>,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscription/confirm?subscription_token={}",
        base_url.get_ref().0,
        token
    );
    email_client
        .send_email(
            &newsubscriber.email,
            "Welcome!",
            &format!(
                "Welcome to our newsletter!<br />\
                    Click <a href=\"{}\">here</a> to confirm your subscription.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
                confirmation_link
            ),
        )
        .await
}

impl TryFrom<SubscriptionForm> for NewSubscriber {
    type Error = String;
    fn try_from(value: SubscriptionForm) -> Result<Self, Self::Error> {
        let name = SuscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;

        Ok(NewSubscriber { name, email })
    }
}

pub fn generate_random_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name = "Store token for subscription", skip(transaction, id, token))]
pub async fn store_token(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"INSERT INTO subscriptions_tokens (subscriptions_tokens ,subscriptions_id)
    VALUES ($1, $2)"#,
        token,
        id
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Could not store subscription token bacause {}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not store the token")
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    write!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "{}\n", cause)?;
        current = cause.source();
    }
    Ok(())
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
