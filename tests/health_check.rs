use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid;
use zero2prod::{
    configuration::{DatabaseSettings, get_configuration},
    email_client,
    telemetry::{get_subscriber, init_subscriber},
};

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

pub struct TestApp {
    pub adress: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listner = TcpListener::bind("127.0.0.1:0").expect("Should have bind the listner");
    let port = listner.local_addr().unwrap().port();
    let adress = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to get config");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    let pool = configure_database(&configuration.database).await;

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let email_client = email_client::EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    let server = zero2prod::startup::run(listner, pool.clone(), email_client)
        .expect("Should have open the app");

    let _ = actix_web::rt::spawn(server);

    TestApp {
        adress,
        db_pool: pool,
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

#[actix_web::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let adress = format!("{}/health_check", app.adress);
    let client = reqwest::Client::new();

    let response = client
        .get(adress)
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

#[actix_web::test]
async fn subscribe_return_a_200_is_ok() {
    let client = reqwest::Client::new();
    let body = "name=le%20gun&email=ursula_le_guin%40gmail.com";
    let app = spawn_app().await;
    let configuration = get_configuration().expect("Failed to get the configuration");

    let _connexion = PgConnection::connect_with(&configuration.database.without_db())
        .await
        .expect("Should have been able to connect");

    let response = client
        .post(&format!("{}/subscription", &app.adress))
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Should have send the request");

    assert_eq!(200, response.status().as_u16());
}

#[actix_web::test]
async fn subscribe_return_400_is_not_ok() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let tests: Vec<(&str, &str)> = vec![
        ("name=sdfsdf", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing name"),
        ("", "Missing email and name"),
        ("name=Ursula&email=definetelynotanemail", "fake email"),
    ];
    for (test, error) in tests {
        let response = client
            .post(&format!("{}/subscription", &app.adress))
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(test)
            .send()
            .await
            .expect("Should have send the request");
        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did not fail as error 400 while it should have as : {}",
            error
        );
    }
}

#[actix_web::test]
async fn subscribe_return_a_200_is_ok_and_saves_data() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let _configuration = get_configuration().expect("Could not fetch configuration");

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscription", &app.adress))
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Should have send the request");

    assert_eq!(200, response.status().as_u16());
    assert_eq!(Some(0), response.content_length());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool.clone())
        .await
        .expect("Should have been able to fetch email and name from subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}
