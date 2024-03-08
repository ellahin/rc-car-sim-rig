use std::time::SystemTime;

use common_data::server::data::telementry::Telementry;

pub struct UDPSession {
    pub stream: Vec<u8>,
    pub username: Option<String>,
    pub telementry: Option<Telementry>,
    pub last_changed: SystemTime,
}
