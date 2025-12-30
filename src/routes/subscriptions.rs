use crate::domain::{NewSubscriber, SubscriberEmail, SuscriberName};
use crate::email_client::EmailClient;
use actix_web::{HttpResponse, Responder, post, web};
use chrono::Utc;
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
    skip(form, connection, email_client),
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
) -> impl Responder {
    let newsubscriber: NewSubscriber = match form.0.try_into() {
        Ok(newsubscriber) => newsubscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    if insert_suscriber(&connection, &newsubscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_email(&email_client, newsubscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Start subscription querry", skip(pool, newsubscriber))]
pub async fn insert_suscriber(
    pool: &PgPool,
    newsubscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status_subscription)
                    VALUES($1, $2, $3, $4, 'confirmed')"#,
        Uuid::new_v4(),
        newsubscriber.email.as_ref(),
        newsubscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Could not subscribe bacause {}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Send an email", skip(email_client, newsubscriber))]
pub async fn send_email(
    email_client: &EmailClient,
    newsubscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
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
