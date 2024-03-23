use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthStartJson {
    pub emailaddress: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthVerifyJson {
    pub auth_code: String,
    pub jwt: String,
}
