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

#[derive(Serialize, Deserialize, Debug)]
pub enum CarState {
    Offline,
    Online,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Car {
    pub uuid: String,
    pub status: CarState,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCars {
    pub cars: Vec<Car>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateCar {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateCarReturn {
    pub name: String,
    pub uuid: String,
    pub api_key: String,
}
