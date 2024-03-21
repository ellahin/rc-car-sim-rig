use std::net::SocketAddr;
use std::sync::mpsc::Sender;

pub async fn handle_udp(buff: [u8; 4096], addr: SocketAddr, udp_tx: Sender<(Vec<u8>, SocketAddr)>) {
}
