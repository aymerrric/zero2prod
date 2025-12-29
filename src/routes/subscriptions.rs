use crate::domain::{NewSubscriber, SubscriberEmail, SuscriberName};
use actix_web::{HttpResponse, Responder, post, web};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use tracing;
use unicode_segmentation::UnicodeSegmentation;
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
    let name = match SuscriberName::parse(form.0.name) {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let email = match SubscriberEmail::parse(form.0.email){
        Ok(email) => email,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let newsubscriber = NewSubscriber {
        name,
        email
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
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at)
                    VALUES($1, $2, $3, $4)"#,
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

pub fn is_valid(name: &str) -> bool {
    let is_empty = name.trim().is_empty();
    let is_too_long = name.graphemes(true).count() > 256;

    let forbidden_characters = ['/', '{', '}', '(', ')', '/', '\\', '"'];

    let contains_forbidden_characters = name.chars().any(|g| forbidden_characters.contains(&g));

    !(contains_forbidden_characters || is_too_long || is_empty)
}
