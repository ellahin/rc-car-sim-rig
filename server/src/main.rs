mod data;
mod lib;
mod repo;

use dotenvy::dotenv;

use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::SqlitePool;

use actix_web::App;
use actix_web::HttpServer;

use lettre::SmtpTransport;

use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() {
    // Setting up global states
    let cars_raw: HashMap<String, data::state::CarConnection> = HashMap::new();
    let cars = Arc::new(Mutex::new(cars_raw));

    let cars_cleanup = Arc::clone(&cars);
    tokio::spawn(async move {
        loop {
            let now = SystemTime::now();
            let offset = now - Duration::from_secs(30);
            let mut cars_lock = cars_cleanup.lock().unwrap();
            let mut to_delete: Vec<String> = Vec::new();

            for (id, car) in cars_lock.iter().clone() {
                if car.last_changed <= offset {
                    to_delete.push(id.clone());
                }
            }

            for delete in to_delete.iter() {
                let _ = cars_lock.remove(delete);
            }
            let _ = tokio::time::sleep(Duration::from_secs(30));
        }
    });

    let userauth_raw: HashMap<String, crate::data::state::UserAuth> = HashMap::new();
    let userauth = Arc::new(Mutex::new(userauth_raw));

    let userauth_cleanup = Arc::clone(&userauth);

    tokio::spawn(async move {
        loop {
            let now = SystemTime::now();
            let offset = now - Duration::from_secs(900);
            let mut userauth_lock = userauth_cleanup.lock().unwrap();
            let mut to_delete: Vec<String> = Vec::new();

            for (id, user) in userauth_lock.iter().clone() {
                if user.created <= offset {
                    to_delete.push(id.clone());
                }
            }

            for delete in to_delete.iter() {
                let _ = userauth_lock.remove(delete);
            }
            let _ = tokio::time::sleep(Duration::from_secs(30));
        }
    });

    dotenv().expect(".env file not found");

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

    if !sqlx::Sqlite::database_exists(&database_url).await.unwrap() {
        sqlx::Sqlite::create_database(&database_url).await.unwrap();
    }

    let migration_path = Path::new("./migrations");

    let sql_pool = SqlitePool::connect(&database_url).await.unwrap();

    Migrator::new(migration_path)
        .await
        .unwrap()
        .run(&sql_pool)
        .await
        .unwrap();

    // Http server
    let http_sql = sql_pool.clone();

    let http_cars = cars.clone();
    let http_userauth = userauth.clone();
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
        sqlx: http_sql,
        cars: http_cars,
        user_auth: http_userauth,
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
    })
    .disable_signals()
    .bind(("127.0.0.1", 8080))
    .expect("cannot bind to port")
    .run()
    .await;
}
