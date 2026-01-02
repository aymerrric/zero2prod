use crate::helpers::{ConfirmationLinks, TestApp, spawn_app};
use wiremock::matchers::{any, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_web::test]
pub async fn should_not_send_mail_to_non_confirmed_user() {
    let app = spawn_app().await;
    create_unconfirmed_user(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = send_email(&app).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
pub async fn send_email_to_confirmed_user() {
    let app = spawn_app().await;
    create_confirmed_user(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = send_email(&app).await;
    assert_eq!(response.status().as_u16(), 200);
}

pub async fn create_unconfirmed_user(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _guard = Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Create unsuscribe user")
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscription(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    app.get_confirmation_links(email_request)
}

pub async fn create_confirmed_user(app: &TestApp) {
    let links: ConfirmationLinks = create_unconfirmed_user(app).await;

    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

pub async fn send_email(app: &TestApp) -> reqwest::Response {
    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
    });
    reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.")
}

#[actix_web::test]
pub async fn send_mail_incomplete_should_fail() {
    let app = spawn_app().await;

    let newsletter_request_bodies = [
        (
            serde_json::json!({

            "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
            }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
            "title": "Newsletter title",
            "content": {

            "html": "<p>Newsletter body as HTML</p>",
            }
            }),
            "missing text",
        ),
        (
            serde_json::json!({
            "title": "Newsletter title",
            "content": {
            "text": "Newsletter body as plain text",

            }
            }),
            "missing html",
        ),
    ];

    let client = reqwest::Client::new();

    for (body, message) in newsletter_request_bodies {
        let response = client
            .post(&format!("{}/newsletter", &app.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did not fail with error 400 when email to send was incomplete as {}",
            message
        );
    }
}
