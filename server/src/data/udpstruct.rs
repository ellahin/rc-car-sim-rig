use std::net::SocketAddr;
use std::time::SystemTime;

use common_data::server::data::telementry::Telementry;

pub struct UDPSession {
    pub socket: SocketAddr,
    pub username: Option<String>,
    pub telementry: Option<Telementry>,
    pub last_changed: SystemTime,
}
