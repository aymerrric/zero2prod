use crate::helpers::{spawn_app};
use zero2prod::configuration::get_configuration;
use sqlx::{PgConnection, Connection};

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
