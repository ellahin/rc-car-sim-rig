use common_data::server::data::telementry::Telementry;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use sqlx::sqlite::Sqlite;
use sqlx::Pool;

use lettre::SmtpTransport;

pub struct CarConnection {
    pub username: Option<String>,
    pub telementry: Option<Telementry>,
    pub last_changed: SystemTime,
}

pub struct UserAuth {
    pub code: String,
    pub created: SystemTime,
}

pub struct HttpState {
    pub sqlx: Pool<Sqlite>,
    pub cars: Arc<Mutex<HashMap<String, CarConnection>>>,
    pub user_auth: Arc<Mutex<HashMap<String, UserAuth>>>,
    pub jwt_secret: String,
    pub smtp_transport: SmtpTransport,
    pub from_address: String,
}
