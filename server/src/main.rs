mod data;
mod lib;
mod repo;

use dotenvy::dotenv;
use repo::database::base::DataBase;
use repo::database::postgres::PostgresDatabase;

use actix_web::App;
use actix_web::HttpServer;

use lettre::SmtpTransport;

use std::env;

#[tokio::main]
async fn main() {
    // Setting up global states
    match dotenv() {
        Ok(_) => (),
        Err(_) => println!("Warning: Dotenv file not found"),
    };

    let jwt_secret = env::var("JWT_SECRETN").expect("JWT_SECRETN not in environment vars");

    let from_address = env::var("FROM_ADDRESS").expect("No FROM_ADDRESS in environment vars");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not in environment vars");

    let database = PostgresDatabase::new(database_url)
        .await
        .expect("Cannot connect to database");

    let smtp_address = match env::var("SMTP_ADDRESS") {
        Err(_) => "127.0.0.1".to_string(),
        Ok(v) => v,
    };

    let smtp_transport = match SmtpTransport::starttls_relay(&smtp_address) {
        Ok(s) => s,
        Err(_) => panic!("Cannot connect to SMTP relay"),
    };

    let http_state = crate::data::state::HttpState {
        database: *database,
        jwt_secret: jwt_secret,
        smtp_transport: smtp_transport.build(),
        from_address: from_address,
    };

    let web_data = actix_web::web::Data::new(http_state);

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web_data.clone())
            .service(crate::repo::http::index::get)
            .service(crate::repo::http::auth::email::put)
            .service(crate::repo::http::auth::email::verify)
            .service(crate::repo::http::user::cars::get)
            .service(crate::repo::http::user::cars::add)
            .service(crate::repo::http::user::cars::remove)
    })
    .disable_signals()
    .bind(("127.0.0.1", 8080))
    .expect("cannot bind to port")
    .run()
    .await;
}
