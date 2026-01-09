use crate::utils::e500;
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;
use crate::startup::ApplicationBaseUrl;

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
    _base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .append_header((LOCATION, "/login"))
            .finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>
<body>
<p>Welcome {username}!</p>
<p>Available actions:</p>
<ol>
<li><a href="/admin/change/password">Change password</a></li>
<li>
<form name="logoutForm" action="/admin/logout" method="post">
<input type="submit" value="Logout">
</form>
</li>
</ol>
</body>
</html>"#,
        )))
}

#[tracing::instrument(name = "Search for username", skip(user_id, pool), fields(username=tracing::field::Empty, user_id=%user_id))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let username = sqlx::query!(
        r#"SELECT username FROM users
    WHERE user_id=$1"#,
        user_id
    )
    .fetch_one(pool)
    .await
    .expect("Could not find the user")
    .username;
    tracing::Span::current().record("username", &tracing::field::display(&username));
    Ok(username)
}

pub async fn clear_session() {}
