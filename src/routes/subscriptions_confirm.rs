use actix_web::{HttpResponse, Responder, web};
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
) -> impl Responder {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let id = match get_subscriber_id_from_token(
        &parameters.into_inner().subscription_token,
        &mut transaction,
    )
    .await
    {
        Ok(opt) => opt,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        None => return HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if confirm_token(subscriber_id, &mut transaction)
                .await
                .is_err()
            {
                return HttpResponse::InternalServerError().finish();
            }
        }
    }
    if transaction.commit().await.is_err() {
        HttpResponse::InternalServerError().finish()
    } else {
        HttpResponse::Ok().finish()
    }
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
