use crate::helpers::{ConfirmationLinks, TestApp, spawn_app};
use uuid::Uuid;
use wiremock::matchers::{any, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_web::test]
pub async fn should_not_send_mail_to_non_confirmed_user() {
    let app = spawn_app().await;
    create_unconfirmed_user(&app).await;
    let (username, password) = app.get_test_user().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .basic_auth(&username, Some(&password))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
pub async fn send_email_to_confirmed_user() {
    let app = spawn_app().await;
    let (username, password): (String, String) = app.get_test_user().await;

    create_confirmed_user(&app).await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .basic_auth(&username, Some(&password))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

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

#[actix_web::test]
pub async fn send_mail_incomplete_should_fail() {
    let app = spawn_app().await;
    let (username, password): (String, String) = app.get_test_user().await;
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
            .basic_auth(&username, Some(&password))
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

#[actix_web::test]
pub async fn without_authentification_should_be_rejected() {
    let app = spawn_app().await;
    let response = reqwest::Client::new()
        .post(format!("{}/newsletter", &app.address))
        .json(&serde_json::json!({
                "title": "Newsletter title",
                "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>"
                }
                }))
        .send()
        .await
        .expect("Could not send the request");
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}
