use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EmailAuthStartJwt {
    pub email: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthJwt {
    pub email: String,
    pub signin_date: i64,
    pub iat: i64,
    pub exp: i64,
}
