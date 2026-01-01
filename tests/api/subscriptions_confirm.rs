use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_web::test]
pub async fn confirmation_without_token_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .get(format!("{}/subscription/confirm", &app.address))
        .send()
        .await
        .expect("Should have send message");
    assert_eq!(response.status().as_u16(), 400);
}

#[actix_web::test]
pub async fn the_link_returned_by_subscriptions_return_200_if_called() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscription(body.to_string()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let links = app.get_confirmation_links(email_request);
    let text_link = links.html;
    // Let's make sure we don't call random APIs on the web
    assert_eq!(text_link.host_str().unwrap(), "127.0.0.1");
    let response = reqwest::get(text_link)
        .await
        .expect("Failed to send request");
    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
pub async fn clicking_on_confirmation_link_should_switch_status() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscription(body.to_string()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let links = app.get_confirmation_links(email_request);
    let text_link = links.html;
    // Let's make sure we don't call random APIs on the web
    assert_eq!(text_link.host_str().unwrap(), "127.0.0.1");
    reqwest::get(text_link)
        .await
        .expect("Failed to send request");

    let saved = sqlx::query!("SELECT status_subscription FROM subscriptions",)
        .fetch_one(&app.db_pool.clone())
        .await
        .expect("Should have been able to fetch email and name from subscription");

    assert_eq!(saved.status_subscription, "confirmed");
}
