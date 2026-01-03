use crate::telemetry::spawn_blocking_track_in_span;
use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::subscriptions::error_chain_fmt,
};

use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{StatusCode, header};
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;
#[derive(Deserialize)]
pub struct Credential {
    username: String,
    password: Secret<String>,
}

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

#[tracing::instrument(
    name = "Publish newsletter",
    skip(connection, email_client, mail_to_send, request)
)]
#[actix_web::post("/newsletter")]
pub async fn newsletter(
    connection: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    mail_to_send: web::Json<Mail>,
    request: HttpRequest,
) -> Result<HttpResponse, NewsletterError> {
    let credential = basic_auth(&request.headers()).map_err(NewsletterError::AuthError)?;

    tracing::Span::current().record("username", &tracing::field::display(&credential.username));
    let id = validate_credential(&connection, credential).await?;
    tracing::Span::current().record("id", &tracing::field::display(&id));
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
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentification failed")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for NewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for NewsletterError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            NewsletterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            NewsletterError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }
    fn error_response(&self) -> HttpResponse {
        match self {
            NewsletterError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            NewsletterError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    // actix_web::http::header provides a collection of constants
                    // for the names of several well-known/standard HTTP headers
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

fn basic_auth(header: &HeaderMap) -> Result<Credential, anyhow::Error> {
    let mut value = header
        .get("Authorization")
        .context("The request does not contain authentication")?
        .to_str()
        .context("Authorization is not basic utf 8")?;

    value = value
        .strip_prefix("Basic ")
        .context("Header does not contain Basic ")?;

    let value = BASE64_STANDARD
        .decode(value)
        .context("Failed to decode Basic credentials")?;
    let decoded_credential =
        String::from_utf8(value).context("Decoded credential are not valid utf8")?;
    let mut credential = decoded_credential.splitn(2, ":");
    let username = credential
        .next()
        .context("Basic does not contain a username")?
        .to_string();
    let password = credential
        .next()
        .context("Basic does not contain a password")?
        .to_string();
    Ok(Credential {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(connection, credential))]
pub async fn validate_credential(
    connection: &PgPool,
    credential: Credential,
) -> Result<uuid::Uuid, NewsletterError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
    "$argon2id$v=19$m=15000,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string());

    if let Some((stored_id, pasword_hash)) =
        get_stored_credential(&credential.username, &connection)
            .await
            .map_err(NewsletterError::UnexpectedError)?
            {
                user_id = Some(stored_id);
                expected_password_hash = pasword_hash;
            }

    spawn_blocking_track_in_span(move || {
        verify_password(expected_password_hash, credential.password)
    })
    .await
    .context("Failed to spawn a blocking task")
    .map_err(NewsletterError::UnexpectedError)?
    .await
    .context("Invalid password")
    .map_err(NewsletterError::AuthError)?;

    user_id.ok_or_else(|| NewsletterError::AuthError(anyhow::anyhow!("Invalid username")))
}

#[tracing::instrument(name = "Verify the password")]
pub async fn verify_password(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), NewsletterError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(NewsletterError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(NewsletterError::AuthError)?;
    Ok(())
}

#[tracing::instrument(name = "get credential from database", skip(username, pool))]
pub async fn get_stored_credential(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let record = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE
                                    username = $1"#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform querry to validate authentification")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(record)
}
