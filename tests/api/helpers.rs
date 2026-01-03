use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use wiremock::MockServer;
use zero2prod::configuration::{DatabaseSettings, get_configuration};
use zero2prod::startup::{Application, get_connection_pool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let mut configuration = get_configuration().expect("Failed to get config");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    configuration.application.port = 0;
    configuration.email_client.base_url = email_server.uri();
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build app");
    let port = application.port();
    configure_database(&configuration.database).await;
    let address = format!("http://127.0.0.1:{}", port);
    let _ = actix_web::rt::spawn(application.run_until_stop());
    let app = TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: port,
    };
    add_test_user(&app.db_pool).await;
    app
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Should have created the pool");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Could not do the migration");

    connection_pool
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscription", &self.address))
            .body(body)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to send subscritpion")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = email_request
            .body_json()
            .expect("Failed to read the body of the mail request");
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1, "Could not parse the link");
            let mut link = Url::parse(links[0].as_str()).expect("failed to parse the link");
            link.set_port(Some(self.port)).unwrap();
            link
        };
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        ConfirmationLinks { plain_text, html }
    }

    pub async fn get_test_user(&self) -> (String, String){
        let user = sqlx::query!(r#"SELECT username, password FROM users"#).fetch_one(&self.db_pool).await.expect("Failed to create the user");
        (user.username, user.password)
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}


pub async fn add_test_user(connection : &PgPool){
    sqlx::query!(r#"INSERT INTO users (user_id, username, password)
                    VALUES ($1, $2, $3)"#, Uuid::new_v4(), Uuid::new_v4().to_string(), Uuid::new_v4().to_string()).execute(connection).await.expect("Could not add user to the database");
}