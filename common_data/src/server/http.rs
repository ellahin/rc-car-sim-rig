use crate::server::json::http::{AuthStartJson, AuthVerifyJson};
use std::str::FromStr;
use std::time::SystemTime;

use serde_json;

use reqwest;
use reqwest::Error;

use email_address::*;

pub struct Http {
    server_address: String,
    auth_token: Option<String>,
    auth_scope: Option<AuthScope>,
}

pub struct AuthScope {
    valid_to: SystemTime,
    email: EmailAddress,
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

    async fn auth_start(&mut self, email: String) -> Result<(), HttpErrors> {
        let client = reqwest::Client::new();

        let auth_json_object = AuthStartJson {
            emailaddress: email,
        };

        let auth_json = serde_json::to_string(&auth_json_object).unwrap();

        let request_url = self.server_address.clone() + "/auth/email";

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

        self.auth_token = Some(token.clone());

        return Ok(());
    }

    async fn auth_verify(&mut self, auth_token: String) -> Result<String, HttpErrors> {
        if self.auth_token.is_none() {
            return Err(HttpErrors::Unauthorized);
        }

        let client = reqwest::Client::new();

        let auth_json_object = AuthVerifyJson {
            auth_code: auth_token,
            jwt: self.auth_token.clone().unwrap(),
        };

        let auth_token = serde_json::to_string(&auth_json_object).unwrap();

        let request_url = self.server_address.clone() + "/auth/email/verify";

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
