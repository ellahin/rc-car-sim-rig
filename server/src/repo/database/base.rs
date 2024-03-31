use common_data::server::json::http::Car;

use chrono::prelude::*;

#[derive(Debug)]
pub enum DatabaseError {
    ServerError,
    DoesNotExist,
    QueryError,
    ConnectionError,
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
    async fn ping_car_state(&self, car_id: String) -> Result<(), DatabaseError>;
}

#[derive(Debug, Clone)]
pub struct UserAuth {
    pub code: String,
    pub created: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub last_login: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct CarFull {
    pub uuid: String,
    pub name: String,
    pub secret: String,
    pub username: String,
    pub last_updated: NaiveDateTime,
    pub last_ping: Option<NaiveDateTime>,
}
