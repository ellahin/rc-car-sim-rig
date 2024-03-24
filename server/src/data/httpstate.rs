use crate::data::udpstruct::UDPSession;
use crate::data::userauthstruct::UserAuth;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sqlx::sqlite::Sqlite;
use sqlx::Pool;

use lettre::SmtpTransport;

pub struct HttpState {
    pub sqlx: Pool<Sqlite>,
    pub cars: Arc<Mutex<HashMap<String, UDPSession>>>,
    pub user_auth: Arc<Mutex<HashMap<String, UserAuth>>>,
    pub jwt_secret: String,
    pub smtp_transport: SmtpTransport,
    pub from_address: String,
}
