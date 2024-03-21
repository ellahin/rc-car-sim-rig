use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

pub async fn handle_udp(buff: [u8; 4096], addr: SocketAddr, udp_tx: Sender<(Vec<u8>, SocketAddr)>) {
    match buff[0] {
        0 => ping(addr, udp_tx).await,
        _ => return,
    }
}

pub async fn ping(addr: SocketAddr, udp_tx: Sender<(Vec<u8>, SocketAddr)>) {
    let ping_string = "pong";
    let _ = udp_tx.send((ping_string.as_bytes().to_vec(), addr)).await;
}
