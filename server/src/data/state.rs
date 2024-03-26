use crate::repo::database::sqlite::SqliteDatabase;

use lettre::SmtpTransport;

pub struct HttpState {
    pub database: SqliteDatabase,
    pub jwt_secret: String,
    pub smtp_transport: SmtpTransport,
    pub from_address: String,
}
