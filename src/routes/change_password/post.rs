use crate::authentication::UserId;
use crate::session_state::TypedSession;
use actix_web::{HttpResponse, ResponseError, http::header::LOCATION, web};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordHasher;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct PasswordForm {
    pub password: Secret<String>,
    #[serde(rename = "confirmpassword")]
    pub confirm_password: Secret<String>,
}
#[tracing::instrument(name = "change password", skip(form, user_id, pool))]
pub async fn password_change(
    form: web::Form<PasswordForm>,
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ChangePasswordError> {
    let user_id = user_id.into_inner();

    if &form.confirm_password.expose_secret() != &form.password.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - \
the field values must match."
                .to_string(),
        )
        .send();
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/admin/change/password"))
            .finish());
    }
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(&form.password.expose_secret().as_bytes(), &salt)
        .expect("Could not hash properly")
        .to_string();

    sqlx::query!(
        r#"UPDATE users
    SET password_hash=$1
    WHERE user_id=$2"#,
        password_hash,
        *user_id
    )
    .execute(pool.as_ref())
    .await
    .context("Could not modify the database")?;
    return Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/admin/dashboard"))
        .finish());
}

#[derive(thiserror::Error, Debug)]
pub enum ChangePasswordError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for ChangePasswordError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }
}
