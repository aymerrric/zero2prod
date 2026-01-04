use argon2::Argon2;
use argon2::PasswordHasher;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use once_cell::sync::Lazy;
use reqwest::Url;
use reqwest::redirect;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{DatabaseSettings, get_configuration};
use zero2prod::startup::{Application, get_connection_pool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

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
    let user = TestUser::generate();
    let api_client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(redirect::Policy::none())
        .build()
        .unwrap();
    let app = TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: port,
        user,
        api_client,
    };
    app.user.store(&app.db_pool).await;
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

pub struct TestUser {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, connection: &PgPool) {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(&self.password.as_bytes(), &salt)
            .expect("Could not hash properly")
            .to_string();

        sqlx::query!(
            r#"INSERT INTO users (user_id, username, password_hash)
        VALUES ($1, $2, $3)"#,
            &self.user_id,
            &self.username,
            password_hash
        )
        .execute(connection)
        .await
        .expect("Could not create a new user");
    }
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_logic<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/login", &self.address))
            .form(&body)
            .send()
            .await
            .expect("Could not send request")
    }

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

    pub async fn get_test_user(&self) -> (String, String) {
        (
            self.user.username.to_string(),
            self.user.password.to_string(),
        )
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", &self.address))
            .send()
            .await
            .expect("Could not send the request")
            .text()
            .await
            .expect("Could not load the html")
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub fn asser_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location)
}
