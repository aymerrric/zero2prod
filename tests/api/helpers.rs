use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
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

    let mut configuration = get_configuration().expect("Failed to get config");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    configuration.application.port = 0;
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build app");
    configure_database(&configuration.database).await;
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = actix_web::rt::spawn(application.run_until_stop());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
    }
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
}
