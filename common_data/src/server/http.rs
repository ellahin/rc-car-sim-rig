use crate::server::json::http::{
    AuthStartJson, AuthVerifyJson, Car, CreateCar, CreateCarReturn, GetCars,
};
use std::str::FromStr;
use std::time::SystemTime;

use serde_json;

use reqwest;
use reqwest::Error;

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

        return Ok(token);
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

    async fn get_cars(&mut self) -> Result<Vec<Car>, HttpErrors> {
        if self.auth_token.is_none() {
            return Err(HttpErrors::Unauthorized);
        }

        let client = reqwest::Client::new();

        let request_url = self.server_address.clone() + "/user/cars";

        let reqwest_raw = client
            .get(request_url)
            .header("Authorization", self.auth_token.clone().unwrap())
            .send()
            .await;

        if reqwest_raw.is_err() {
            let err = reqwest_raw.unwrap_err();

            let error_code = err.status().unwrap();

            if error_code == 401 {
                return Err(HttpErrors::AuthError);
            }
            return Err(HttpErrors::ServerError);
        }

        let res = reqwest_raw.unwrap();

        let headers = res.headers();

        let auth_token = headers.get("Authorization");

        if auth_token.is_some() {
            self.auth_token = Some(auth_token.unwrap().to_str().unwrap().to_string());
        }

        let cars_raw = res.text().await;

        if cars_raw.is_err() {
            return Err(HttpErrors::ServerError);
        }

        let cars: GetCars = serde_json::from_str(&cars_raw.unwrap()).unwrap();

        return Ok(cars.cars);
    }

    async fn create_car(&mut self, car: CreateCar) -> Result<CreateCarReturn, HttpErrors> {
        if self.auth_token.is_none() {
            return Err(HttpErrors::Unauthorized);
        }

        let client = reqwest::Client::new();

        let request_url = self.server_address.clone() + "/user/cars/add";

        let put_string = serde_json::to_string(&car).unwrap();

        let reqwest_raw = client
            .put(request_url)
            .body(put_string)
            .header("Authorization", self.auth_token.clone().unwrap())
            .send()
            .await;

        if reqwest_raw.is_err() {
            let err = reqwest_raw.unwrap_err();

            let error_code = err.status().unwrap();

            if error_code == 401 {
                return Err(HttpErrors::AuthError);
            }
            return Err(HttpErrors::ServerError);
        }

        let res = reqwest_raw.unwrap();

        let headers = res.headers();

        let auth_token = headers.get("Authorization");

        if auth_token.is_some() {
            self.auth_token = Some(auth_token.unwrap().to_str().unwrap().to_string());
        }

        let cars_raw = res.text().await;

        if cars_raw.is_err() {
            return Err(HttpErrors::ServerError);
        }

        let car_res: CreateCarReturn = serde_json::from_str(&cars_raw.unwrap()).unwrap();

        return Ok(car_res);
    }
}
