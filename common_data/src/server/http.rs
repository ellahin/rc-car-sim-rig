use crate::server::json::http::{AuthStartJson, AuthVerifyJson};
use std::time::SystemTime;

use serde_json;

use reqwest;
use reqwest::Error;

pub struct Http {
    server_address: String,
    auth_token: Option<String>,
    auth_scope: Option<AuthScope>,
}

pub struct AuthScope {
    valid_to: SystemTime,
    email: String,
}

pub enum HttpErrors {
    ServerError,
    AuthError,
    BadRequest,
    Unauthorized,
    NotFound,
}

impl Http {
    fn new(server_address: String) -> Http {
        return Http {
            server_address,
            auth_token: None,
            auth_scope: None,
        };
    }

    async fn auth_start(&self, email: String) -> Result<String, HttpErrors> {
        let client = reqwest::Client::new();

        let auth_json_object = AuthStartJson {
            emailaddress: email,
        };

        let auth_json = serde_json::to_string(&auth_json_object).unwrap();

        let request_url = self.server_address.clone() + "/authstart";

        let reqwest_raw = client.post(request_url).body(auth_json).send().await;

        if reqwest_raw.is_err() {
            return Err(HttpErrors::BadRequest);
        }

        let reqwest = reqwest_raw.unwrap();

        let token_raw = reqwest.text().await;

        if token_raw.is_err() {
            return Err(HttpErrors::ServerError);
        }

        let token = token_raw.unwrap();

        return Ok(token);
    }

    async fn auth_verify(&mut self, auth_token: String) -> Result<String, HttpErrors> {
        let client = reqwest::Client::new();

        let auth_json_object = AuthVerifyJson {
            auth_code: auth_token,
        };

        let auth_token = serde_json::to_string(&auth_json_object).unwrap();

        let request_url = self.server_address.clone() + "/authverify";

        let reqwest_raw = client.post(request_url).body(auth_token).send().await;

        if reqwest_raw.is_err() {
            return Err(HttpErrors::AuthError);
        }

        let reqwest = reqwest_raw.unwrap();

        let token_raw = reqwest.text().await;

        if token_raw.is_err() {
            return Err(HttpErrors::ServerError);
        }

        let token = token_raw.unwrap();

        return Ok(token);
    }
}
