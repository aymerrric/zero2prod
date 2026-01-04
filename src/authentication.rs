use crate::telemetry::spawn_blocking_track_in_span;
use actix_web::ResponseError;
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for AuthError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidCredentials(_) => actix_web::http::StatusCode::UNAUTHORIZED,
        }
    }
}

#[tracing::instrument(name = "Validate credentials", skip(connection, credential))]
pub async fn validate_credential(
    connection: &PgPool,
    credential: Credential,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
    gZiV/M1gPc22ElAH/Jh1Hw$\
    CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_id, pasword_hash)) =
        get_stored_credential(&credential.username, &connection)
            .await
            .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_id);
        expected_password_hash = pasword_hash;
    }

    spawn_blocking_track_in_span(move || {
        verify_password(expected_password_hash, credential.password)
    })
    .await
    .context("Failed to spawn a blocking task")
    .map_err(AuthError::UnexpectedError)?
    .await
    .context("Invalid password")
    .map_err(AuthError::InvalidCredentials)?;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Invalid username")))
}

#[tracing::instrument(name = "Verify the password")]
pub async fn verify_password(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)?;
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

#[derive(Deserialize)]
pub struct Credential {
    pub username: String,
    pub password: Secret<String>,
}
