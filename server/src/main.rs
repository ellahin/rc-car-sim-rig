mod data;
mod repo;

use tokio::{net::UdpSocket, sync::mpsc};

use std::collections::HashMap;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Setting up global states
    let cars: Arc<HashMap<String, data::udpstruct::UDPSession>> = Arc::new(HashMap::new());

    // Setting up udp socket handler
    let socket = UdpSocket::bind("0.0.0.0:8080".parse::<SocketAddr>().unwrap())
        .await
        .expect("cannot bind address");

    let udp_read = Arc::new(socket);
    let udp_write = udp_read.clone();
    let udp_cars = Arc::clone(&cars);

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

    let (udp_tx, mut udp_rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    tokio::spawn(async move {
        while let Some((bytes, addr)) = udp_rx.recv().await {
            let len = udp_write.send_to(&bytes, addr).await.unwrap();
            println!("{:?} bytes sent", len);
        }
    });
}
