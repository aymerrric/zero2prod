use crate::domain::{NewSubscriber, SubscriberEmail, SuscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::{HttpResponse, Responder, post, web};
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
) -> impl Responder {
    let newsubscriber: NewSubscriber = match form.0.try_into() {
        Ok(newsubscriber) => newsubscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let mut transaction: sqlx::Transaction<'_, sqlx::Postgres> = match connection.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let id = match insert_suscriber(&mut transaction, &newsubscriber).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let token = generate_random_token();
    if store_token(&mut transaction, id, &token).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    if send_email(&email_client, newsubscriber, base_url, &token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    match transaction.commit().await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
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
            newsubscriber.email,
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


#[tracing::instrument (name = "Store token for subscription", skip(transaction, id, token))]
pub async fn store_token(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
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
        e
    })?;
    Ok(())
}
