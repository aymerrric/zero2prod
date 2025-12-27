use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

use zero2prod::telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber =
        telemetry::get_subscriber("zero2prod".to_string(), "info".to_string(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(&configuration.database.connection_string())
        .expect("Failed to start the pool conection to the database");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(&address).expect("Should have bind the listener");
    run(listener, connection_pool)?.await
}
