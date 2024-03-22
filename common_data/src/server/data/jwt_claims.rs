use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EmailAuthStartJwt {
    pub email: String,
    pub iat: i64,
    pub exp: i64,
}
