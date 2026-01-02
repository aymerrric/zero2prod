use crate::routes::subscriptions::error_chain_fmt;
use actix_web::{HttpResponse, ResponseError, web};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm subscription", skip(parameters, pool))]
#[actix_web::get("/subscription/confirm")]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a pool connection")?;
    let id = get_subscriber_id_from_token(&parameters.subscription_token, &mut transaction)
        .await
        .context("Failed to get subscriber id")?;
    match id {
        None => return Ok(HttpResponse::Unauthorized().finish()),
        Some(subscriber_id) => {
            confirm_token(subscriber_id, &mut transaction)
                .await
                .context("Failed to confirm the token")?;
        }
    }
    transaction
        .commit()
        .await
        .context("Failed to commit the transaction into the database")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Confirm the id", skip(id, transaction))]
pub async fn confirm_token(
    id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions  SET status_subscription = 'confirmed'
        WHERE id=$1"#,
        id
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update table {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Confirm the id", skip(token, transaction))]
pub async fn get_subscriber_id_from_token(
    token: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<Uuid>, sqlx::Error> {
    let saved = sqlx::query!(
        r#"SELECT subscriptions_id FROM subscriptions_tokens
    WHERE subscriptions_tokens = $1"#,
        token
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(saved.map(|r| r.subscriptions_id))
}

#[derive(thiserror::Error)]
#[error(transparent)]
pub struct ConfirmationError(#[from] anyhow::Error);

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmationError {}
