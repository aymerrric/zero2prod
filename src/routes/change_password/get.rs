use crate::utils::see_other;
use actix_web::{HttpResponse, ResponseError, http::header::ContentType};
use actix_web_flash_messages::IncomingFlashMessages;
use anyhow::Context;

use std::fmt::Write;

use crate::session_state::TypedSession;

pub async fn form_password(
    session: TypedSession,
    flash_message: IncomingFlashMessages,
) -> Result<HttpResponse, ChangePasswordError> {
    if let Some(_id) = session.get_user_id().context("Could not read session")? {
        let mut error_html = String::new();
        for m in flash_message.iter() {
            write!(error_html, "<p><i>{}</p></i>", m.content())
                .context("Failed to parse flash message")?;
        }
        Ok(HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Change Password</title>
</head>
<body>
        {error_html}
 <form action="/admin/change/password" method="post">
        <label for="newpassword"> New Password</label>
        <input type="password" name="password" placeholder="type your new password" id="newpassword">    
        <label for="newpasswordconfirm"> Confirm New Password</label>
        <input type="password" placeholder="confirm new password" name="confirmpassword" id="newpasswordconfirm">
        <button type="submit">Submit</button>
    </form>
<button type="submit">Change password</button>
<p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
            )))
    } else {
        Ok(see_other("/login"))
    }
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
