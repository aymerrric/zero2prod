use crate::domain::{NewSubscriber, SubscriberEmail, SuscriberName};
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
    skip(form, connection),
    fields(
        subscriber_email = %form.email,
        subcriber_name = %form.name
)
)]
#[post("/subscription")]
async fn subscribe(
    form: web::Form<SubscriptionForm>,
    connection: web::Data<PgPool>,
) -> impl Responder {
    let newsubscriber: NewSubscriber = match form.0.try_into() {
        Ok(newsubscriber) => newsubscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_suscriber(&connection, &newsubscriber).await {
        Ok(_) => {
            tracing::info!("Info of the new user have been saved ");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("Could not save the info of the new user because : {:?}", e,);
            HttpResponse::InternalServerError().finish()
        }
    };
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

impl TryFrom<SubscriptionForm> for NewSubscriber {
    type Error = String;
    fn try_from(value: SubscriptionForm) -> Result<Self, Self::Error> {
        let name = SuscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;

        Ok(NewSubscriber { name, email })
    }
}
