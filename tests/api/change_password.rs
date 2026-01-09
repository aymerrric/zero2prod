use crate::helpers::{asser_is_redirect_to, spawn_app};

#[actix_web::test]
pub async fn unauthorized_should_not_be_able_to_reach_get() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    asser_is_redirect_to(&response, "/login");
}

#[actix_web::test]
pub async fn unauthorized_should_not_be_able_to_reach_post() {
    let app = spawn_app().await;
    let body = serde_json::json!({"password" : "random", "confirmpassword" : "random"});
    let response = app.post_change_password(&body).await;
    asser_is_redirect_to(&response, "/login");
}

#[actix_web::test]
pub async fn password_do_not_match_should_be_unnacepted() {
    let app = spawn_app().await;
    let body =
        serde_json::json!({"password" : "random", "confirmpassword" : "random_but_different"});
    app.user.connect(&app).await;
    let response = app.get_change_password().await;
    assert_eq!(response.status().as_u16(), 200);
    let response = app.post_change_password(&body).await;
    asser_is_redirect_to(&response, "/admin/change/password");
    let html = app.get_change_password_html().await;
    assert!(html.contains(
        "<p><i>You entered two different new passwords - the field values must match.</p></i>"
    ));
}

#[actix_web::test]
pub async fn password_should_change_if_done_properly() {
    let app = spawn_app().await;
    app.user.connect(&app).await;
    let body = serde_json::json!({"password" : "samepassword", "confirmpassword" : "samepassword"});
    let response = app.post_change_password(&body).await;
    asser_is_redirect_to(&response, "/admin/dashboard");
    app.user.logout(&app).await;
    let new_credentials =
        serde_json::json!({"username" : app.user.username, "password" : "samepassword"});
    let response = app.post_logic(&new_credentials).await;
    asser_is_redirect_to(&response, "/admin/dashboard");
}
