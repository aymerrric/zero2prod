use crate::helpers::{asser_is_redirect_to, spawn_app};

#[actix_web::test]
pub async fn should_refuse_client_not_logged_in() {
    let app = spawn_app().await;
    let response = app
        .api_client
        .get(format!("{}/admin/dashboard", &app.address))
        .send()
        .await
        .expect("Could not send the request");
    asser_is_redirect_to(&response, "/login");
}

#[actix_web::test]
pub async fn log_out_should_clear_session() {
    let app = spawn_app().await;
    app.user.connect(&app).await;
    app.user.logout(&app).await;
    let response = app.get_admindashboard().await;
    asser_is_redirect_to(&response, "/login");
}
