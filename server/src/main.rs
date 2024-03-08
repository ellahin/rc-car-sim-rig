mod data;
mod repo;

use tokio::{net::UdpSocket, sync::mpsc};

use std::collections::HashMap;
use std::io::{self, Read};
use std::net::SocketAddr;
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

    tokio::spawn(async move {
        loop {
            let mut buff = [0; 4096];

            let incomming = udp_read.recv_from(&mut buff).await;

            if incomming.is_err() {
                let err = incomming.unwrap_err();
                println!("error on udp read: {:?}", err);
                continue;
            }

            let (len, addr) = incomming.unwrap();

            println!("reccived {:?} bytes from {:?}", len, addr);
        }
    });
}
