use crate::server::json::http::{
    AuthStartJson, AuthVerifyJson, Car, CreateCar, CreateCarReturn, GetCars,
};
use std::str::FromStr;
use std::time::SystemTime;

use serde_json;

use reqwest;
use reqwest::Error;
use reqwest::StatusCode;

use email_address::*;

pub struct Http {
    server_address: String,
    auth_token: Option<String>,
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
    TooManyCars,
    EncodeError,
    DecodeError,
}

impl Http {
    fn new(server_address: String) -> Http {
        return Http {
            server_address,
            auth_token: None,
        };
    }

    async fn auth_start(&mut self, email: String) -> Result<String, HttpErrors> {
        let client = reqwest::Client::new();

        let auth_json_object = AuthStartJson {
            emailaddress: email,
        };

        let auth_json = match serde_json::to_string(&auth_json_object) {
            Ok(s) => s,
            Err(_) => return Err(HttpErrors::EncodeError),
        };

        let request_url = self.server_address.clone() + "/auth/email";

        let reqwest = match client.post(request_url).body(auth_json).send().await {
            Ok(r) => r,
            Err(_) => return Err(HttpErrors::BadRequest),
        };

        match reqwest.text().await {
            Ok(t) => Ok(t),
            Err(_) => Err(HttpErrors::ServerError),
        }
    }

    async fn auth_verify(&mut self, auth_token: String) -> Result<String, HttpErrors> {
        let auth_json_object = match self.auth_token.clone() {
            Some(a) => AuthVerifyJson {
                auth_code: auth_token,
                jwt: a.clone(),
            },
            None => return Err(HttpErrors::Unauthorized),
        };

        let client = reqwest::Client::new();

        let auth_token = match serde_json::to_string(&auth_json_object) {
            Ok(s) => s,
            Err(_) => return Err(HttpErrors::EncodeError),
        };

        let request_url = self.server_address.clone() + "/auth/email/verify";

        let reqwest = match client.post(request_url).body(auth_token).send().await {
            Ok(r) => r,
            Err(_) => return Err(HttpErrors::AuthError),
        };

        match reqwest.text().await {
            Ok(t) => Ok(t),
            Err(_) => return Err(HttpErrors::ServerError),
        }
    }

    async fn get_cars(&mut self) -> Result<Vec<Car>, HttpErrors> {
        let auth_token = match self.auth_token.clone() {
            Some(t) => t,
            None => return Err(HttpErrors::Unauthorized),
        };

        let client = reqwest::Client::new();

        let request_url = self.server_address.clone() + "/user/cars";

        let res = match client
            .get(request_url)
            .header("Authorization", auth_token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => match e.status() {
                None => return Err(HttpErrors::ServerError),
                Some(code) => match code.as_u16() {
                    400 => return Err(HttpErrors::BadRequest),
                    401 => return Err(HttpErrors::AuthError),
                    _ => return Err(HttpErrors::ServerError),
                },
            },
        };

        let headers = res.headers();

        match headers.get("Authorization") {
            None => (),
            Some(t) => match t.to_str() {
                Err(_) => return Err(HttpErrors::DecodeError),
                Ok(s) => self.auth_token = Some(s.to_string()),
            },
        };

        let cars: GetCars = match res.text().await {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(o) => o,
                Err(_) => return Err(HttpErrors::DecodeError),
            },
            Err(_) => return Err(HttpErrors::ServerError),
        };

        return Ok(cars.cars);
    }

    async fn create_car(&mut self, car: CreateCar) -> Result<CreateCarReturn, HttpErrors> {
        let auth_token = match self.auth_token.clone() {
            Some(t) => t,
            None => return Err(HttpErrors::Unauthorized),
        };

        let client = reqwest::Client::new();

        let request_url = self.server_address.clone() + "/user/cars/";

        let put_string = match serde_json::to_string(&car) {
            Ok(s) => s,
            Err(_) => return Err(HttpErrors::EncodeError),
        };

        let res = match client
            .put(request_url)
            .body(put_string)
            .header("Authorization", auth_token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => match e.status() {
                None => return Err(HttpErrors::ServerError),
                Some(code) => match code.as_u16() {
                    400 => return Err(HttpErrors::BadRequest),
                    401 => return Err(HttpErrors::AuthError),
                    _ => return Err(HttpErrors::ServerError),
                },
            },
        };

        let headers = res.headers();

        match headers.get("Authorization") {
            None => (),
            Some(t) => match t.to_str() {
                Err(_) => return Err(HttpErrors::DecodeError),
                Ok(s) => self.auth_token = Some(s.to_string()),
            },
        };

        match res.text().await {
            Err(_) => Err(HttpErrors::ServerError),
            Ok(t) => match serde_json::from_str(&t) {
                Ok(o) => Ok(o),
                Err(_) => Err(HttpErrors::DecodeError),
            },
        }
    }

    async fn delete_car(&mut self, car_uuid: String) -> Result<GetCars, HttpErrors> {
        let auth_token = match self.auth_token.clone() {
            Some(t) => t,
            None => return Err(HttpErrors::Unauthorized),
        };

        let client = reqwest::Client::new();

        let request_url = format!("{}/user/cars/{}", self.server_address.clone(), car_uuid);

        let res = match client
            .delete(request_url)
            .header("Authorization", auth_token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => match e.status() {
                None => return Err(HttpErrors::ServerError),
                Some(code) => match code.as_u16() {
                    400 => return Err(HttpErrors::BadRequest),
                    401 => return Err(HttpErrors::AuthError),
                    _ => return Err(HttpErrors::ServerError),
                },
            },
        };

        let headers = res.headers();

        match headers.get("Authorization") {
            None => (),
            Some(t) => match t.to_str() {
                Err(_) => return Err(HttpErrors::DecodeError),
                Ok(s) => self.auth_token = Some(s.to_string()),
            },
        };

        let cars = match res.text().await {
            Ok(t) => t,
            Err(_) => return Err(HttpErrors::ServerError),
        };

        match serde_json::from_str(&cars) {
            Ok(c) => Ok(c),
            Err(_) => Err(HttpErrors::DecodeError),
        }
    }
}
