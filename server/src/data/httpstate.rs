use crate::data::udpstruct::UDPSession;
use crate::data::userauthstruct::UserAuth;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

use sqlx::sqlite::Sqlite;
use sqlx::Pool;

use lettre::SmtpTransport;

pub struct HttpState {
    pub sqlx: Pool<Sqlite>,
    pub cars: Arc<Mutex<HashMap<String, UDPSession>>>,
    pub user_auth: Arc<Mutex<HashMap<String, UserAuth>>>,
    pub udp_tx: Sender<(Vec<u8>, SocketAddr)>,
    pub jwt_secret: String,
    pub smtp_transport: SmtpTransport,
    pub from_address: String,
}
