use common_data::server::data::telementry::Telementry;
use common_data::server::json::http::Car;

use serde::{Deserialize, Serialize};

use chrono::prelude::*;

use async_trait::async_trait;

#[derive(Debug)]
pub enum DatabaseError {
    ServerError,
    DoesNotExist,
    QueryError,
}

pub trait DataBase {
    async fn new(connection_url: String) -> Result<Box<Self>, DatabaseError>;
    async fn user_login(&self, username: String) -> Result<(), DatabaseError>;
    async fn create_user_auth(&self, username: String, code: String) -> Result<(), DatabaseError>;
    async fn fetch_user(&self, username: String) -> Result<Option<User>, DatabaseError>;
    async fn fetch_user_auth(&self, username: String) -> Result<Option<UserAuth>, DatabaseError>;
    async fn delete_user_auth(&self, username: String) -> Result<(), DatabaseError>;
    async fn fetch_cars_by_user(&self, username: String) -> Result<Vec<Car>, DatabaseError>;
    async fn fetch_car(&self, car_id: String) -> Result<Option<CarFull>, DatabaseError>;
    async fn delete_car(&self, car_id: String) -> Result<(), DatabaseError>;
    async fn put_car(&self, car: CarFull) -> Result<(), DatabaseError>;
    async fn put_car_state(&self, car_id: String, car_state: CarState)
        -> Result<(), DatabaseError>;
    async fn get_car_state(&self, car_id: String) -> Result<Option<CarState>, DatabaseError>;
    async fn ping_car_state(&self, car_id: String) -> Result<(), DatabaseError>;
}

#[derive(Debug, Clone)]
pub struct UserAuth {
    pub code: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub last_login: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarState {
    pub username: Option<String>,
    pub telementry: Option<Telementry>,
    pub last_changed: i64,
}

#[derive(Debug, Clone)]
pub struct CarFull {
    pub uuid: String,
    pub status: Option<CarState>,
    pub name: String,
    pub secret: String,
    pub username: String,
}
