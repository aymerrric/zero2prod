use crate::authentication::{AuthError, Credential, validate_credential};
use crate::session_state::TypedSession;
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, ResponseError, web};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument (name="Login", skip(form, pool, session), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credential = Credential {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credential.username));
    match validate_credential(&pool, credential).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.renew();
            session
                .insert_user(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(e) => {
            let e: LoginError = e.into();
            FlashMessage::error(e.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();
            Err(InternalError::from_response(e, response))
        }
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(e, response)
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for LoginError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::SEE_OTHER
    }
}

impl From<AuthError> for LoginError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(e) => Self::AuthError(e),
            AuthError::UnexpectedError(e) => Self::UnexpectedError(e),
        }
    }
}
