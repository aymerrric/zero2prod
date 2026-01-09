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

#[actix_web::test]
pub async fn redirect_to_admin_dashboard_on_success_login() {
    let app = spawn_app().await;

    let body =
        serde_json::json!({"username" : &app.user.username, "password" : &app.user.password});
    let response = app.post_logic(&body).await;
    asser_is_redirect_to(&response, &format!("/admin/dashboard"));
    let html_page = app.get_admindashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", &app.user.username)));
}
