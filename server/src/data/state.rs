use crate::repo::database::postgres::PostgresDatabase;

use lettre::SmtpTransport;

pub struct HttpState {
    pub database: PostgresDatabase,
    pub jwt_secret: String,
    pub smtp_transport: SmtpTransport,
    pub from_address: String,
}
