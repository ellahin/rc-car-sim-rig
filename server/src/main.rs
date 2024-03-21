mod data;
mod repo;

use tokio::{net::UdpSocket, sync::mpsc};

use dotenvy::dotenv;

use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::SqlitePool;

use actix_web::App;
use actix_web::HttpServer;

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() {
    // Setting up global states
    let mut cars_raw: HashMap<String, data::udpstruct::UDPSession> = HashMap::new();
    let mut cars = Arc::new(Mutex::new(cars_raw));

    let mut cars_cleanup = Arc::clone(&cars);
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

    let mut userauth_raw: HashMap<String, crate::data::userauthstruct::UserAuth> = HashMap::new();
    let mut userauth = Arc::new(Mutex::new(userauth_raw));

    let mut userauth_cleanup = Arc::clone(&userauth);
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

    // Setting up udp socket handler
    let socket = UdpSocket::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap())
        .await
        .expect("cannot bind address");

    let udp_read = Arc::new(socket);
    let udp_write = udp_read.clone();
    let udp_cars = Arc::clone(&cars);

    let (udp_tx, mut udp_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    tokio::spawn(async move {
        while let Some((bytes, addr)) = udp_rx.recv().await {
            let len = udp_write.send_to(&bytes, addr).await.unwrap();
            println!("{:?} bytes sent", len);
        }
    });

    let udp_tx_udp_rx = udp_tx.clone();

    tokio::spawn(async move {
        loop {
            let mut buff = [0; 4096];

            let incomming = udp_read.recv_from(&mut buff).await;

            if incomming.is_err() {
                let err = incomming.unwrap_err();
                println!("error on udp read: {:?}", err);
                continue;
            }

            let udp_tx_clone = udp_tx_udp_rx.clone();

            tokio::spawn(async move {
                let (len, addr) = incomming.unwrap();

                crate::repo::handlers::udphandler::handle_udp(buff, addr, udp_tx_clone.clone())
                    .await;

                println!("reccived {:?} bytes from {:?}", len, addr);
            });
        }
    });

    // Http server
    let http_sql = sql_pool.clone();

    let http_cars = cars.clone();
    let http_userauth = userauth.clone();
    let http_udp_tx = udp_tx.clone();

    let http_state = crate::data::httpstate::HttpState {
        sqlx: http_sql,
        cars: http_cars,
        user_auth: http_userauth,
        udp_tx: http_udp_tx,
    };

    let web_data = actix_web::web::Data::new(http_state);

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web_data.clone())
            .service(crate::repo::http::index::get)
    })
    .disable_signals()
    .bind(("127.0.0.1", 8080))
    .expect("cannot bind to port")
    .run()
    .await;
}
