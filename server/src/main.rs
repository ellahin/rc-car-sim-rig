mod data;
mod lib;
mod repo;

use repo::database::base::DataBase;
use repo::database::sqlite::SqliteDatabase;

use dotenvy::dotenv;

use actix_web::App;
use actix_web::HttpServer;

use lettre::SmtpTransport;

use std::env;

#[tokio::main]
async fn main() {
    // Setting up global states
    let dot_env = dotenv();

    if dot_env.is_err() {
        println!("Warning: Dotenv file not found");
    }

    if env::var("JWT_SECRETN").is_err() {
        panic!("JWT_SECRETN not in environment vars");
    }

    let jwt_secret = env::var("JWT_SECRETN").unwrap();

    if env::var("FROM_ADDRESS").is_err() {
        panic!("No FROM_ADDRESS in environment vars")
    }

    if env::var("DATABASE_URL").is_err() {
        panic!("DATABASE_URL not in environment vars");
    }

    let database_url = env::var("DATABASE_URL").unwrap();

    let database = SqliteDatabase::new(database_url)
        .await
        .expect("Cannot connect to database");

    let mut smtp_address = "127.0.0.1".to_string();

    if env::var("SMTP_ADDRESS").is_err() {
        println!("No SMTP_ADDRESS in environment variables, reverting to using 127.0.0.1");
    } else {
        smtp_address = env::var("SMTP_ADDRESS").unwrap();
    }

    let smtp_transport_raw = SmtpTransport::relay(&smtp_address);

    if smtp_transport_raw.is_err() {
        panic!("Cannot connect to SMTP relay");
    }

    let from_address = env::var("FROM_ADDRESS").unwrap();

    let http_state = crate::data::state::HttpState {
        database: *database,
        jwt_secret: jwt_secret,
        smtp_transport: smtp_transport_raw.unwrap().build(),
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
