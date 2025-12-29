use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretBox};
use serde::Serialize;

pub struct EmailClient {
    pub sender: SubscriberEmail,
    pub http_client: Client,
    pub base_url: String,
    pub authorization_token: SecretBox<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: SecretBox<String>,
        timeout : std::time::Duration
    ) -> EmailClient {
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();
        EmailClient {
            base_url,
            sender,
            http_client,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: subject,
            html_body: html_content,
            text_body: content,
        };

        self
            .http_client
            .post(&url)
            .json(&request_body)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {


    use super::*;
    use claim::{assert_err, assert_ok};
    use fake::Faker;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, faker::internet::en::SafeEmail};
    use secrecy::SecretBox;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, header_exists, method, path},
    };

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String{
        Sentence(1..2).fake()
    }
    fn content() -> String{
        Paragraph(1..10).fake()
    }
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url : String) -> EmailClient{
        EmailClient::new(base_url, email(), SecretBox::new(Faker.fake()), std::time::Duration::from_millis(200))
    }

    #[actix_web::test]
    async fn send_email_respond_with_500_should_not_be_ok() {
        let mock_server = wiremock::MockServer::start().await;
        let email_client = email_client(mock_server.uri());
        let response_template =
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_mins(3));
        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(response_template)
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    #[actix_web::test]
    async fn send_email_respond_in_3_minutes_should_be_err() {
        let mock_server = wiremock::MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
    #[actix_web::test]
    async fn send_email_fires_http_request() {
        let mock_server = wiremock::MockServer::start().await;
        let email_client = email_client(mock_server.uri());
       
        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }
}
