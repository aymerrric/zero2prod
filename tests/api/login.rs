use crate::helpers::{asser_is_redirect_to, spawn_app};

#[actix_web::test]
pub async fn an_error_message_is_set_on_failure() {
    let app = spawn_app().await;
    let login_body =
        serde_json::json!({"username" : "random_username", "password" : "random_password"});
    let response = app.post_logic(&login_body).await;
    asser_is_redirect_to(&response, "/login");

    let text = app.get_login_html().await;
    assert!(text.contains(r#"<p><i>Authentication failed</i></p>"#));

    let text = app.get_login_html().await;
    assert!(
        !(text.contains(r#"<p><i>Authentication failed</i></p>"#)),
        "cookie should be used once and disapear"
    );
}
