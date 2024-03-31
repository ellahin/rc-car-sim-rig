use crate::repo::database::base::{CarFull, DataBase, DatabaseError, User, UserAuth};

use common_data::server::json::http::Car;

use std::path::Path;
use std::sync::Arc;

use sqlx::migrate::Migrator;
use sqlx::types::Json;
use sqlx::PgPool;

use chrono::prelude::*;
use chrono::TimeDelta;

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Arc<PgPool>,
}

impl DataBase for PostgresDatabase {
    async fn new(connection_url: String) -> Result<Box<Self>, DatabaseError> {
        let migration_path = Path::new("./migrations");

        let pg_pool_raw = PgPool::connect(&connection_url).await;

        if pg_pool_raw.is_err() {
            return Err(DatabaseError::ConnectionError);
        }

        let pg_pool = pg_pool_raw.unwrap();

        Migrator::new(migration_path)
            .await
            .unwrap()
            .run(&pg_pool)
            .await
            .unwrap();

        return Ok(Box::new(PostgresDatabase {
            pool: Arc::new(pg_pool),
        }));
    }

    async fn user_login(&self, username: String) -> Result<(), DatabaseError> {
        let get_user_raw = sqlx::query!("SELECT username from users where username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        if get_user_raw.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let get_user = get_user_raw.unwrap();

        if get_user.is_none() {
            let _ = sqlx::query!("INSERT INTO users(username) VALUES($1)", username,)
                .execute(&*self.pool)
                .await;
        } else {
            let _ = sqlx::query!(
                "update users set lastsignin = (NOW() at time zone 'utc') where username = $1",
                username
            )
            .execute(&*self.pool)
            .await;
        }

        return Ok(());
    }

    async fn fetch_user_auth(&self, username: String) -> Result<Option<UserAuth>, DatabaseError> {
        let get_auth = sqlx::query!("SELECT * from auth WHERE username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        if get_auth.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let auth_opt = get_auth.unwrap();

        if auth_opt.is_none() {
            return Ok(None);
        }

        let auth = auth_opt.unwrap();

        return Ok(Some(UserAuth {
            code: auth.code.clone(),
            created: auth.timestamp.clone(),
        }));
    }

    async fn create_user_auth(&self, username: String, code: String) -> Result<(), DatabaseError> {
        let auth_fetch = self.fetch_user_auth(username.clone()).await;

        if auth_fetch.is_err() {
            return Err(DatabaseError::ServerError);
        }

        if auth_fetch.unwrap().is_some() {
            let auth_delete = self.delete_user_auth(username.clone()).await;
            if auth_delete.is_err() {
                return Err(DatabaseError::ServerError);
            }
        }

        let query = sqlx::query!(
            "INSERT INTO auth (username, code) VALUES ($1, $2)",
            username,
            code
        )
        .execute(&*self.pool)
        .await;

        if query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        return Ok(());
    }

    async fn fetch_user(&self, username: String) -> Result<Option<User>, DatabaseError> {
        let get_user_raw = sqlx::query!("SELECT * from users where username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        if get_user_raw.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let get_user = get_user_raw.unwrap();

        if get_user.is_some() {
            let temp = get_user.unwrap();
            let object = User {
                username: temp.username.clone(),
                last_login: temp.lastsignin,
            };

            return Ok(Some(object));
        }

        return Ok(None);
    }

    async fn fetch_cars_by_user(&self, username: String) -> Result<Vec<Car>, DatabaseError> {
        let car_query = sqlx::query!("SELECT * from cars where username = $1", username)
            .fetch_all(&*self.pool)
            .await;

        if car_query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let cars = car_query.unwrap();

        let mut return_cars: Vec<Car> = Vec::new();

        let offset = (Utc::now() - TimeDelta::try_minutes(2).unwrap()).naive_utc();

        for car in cars {
            if car.last_ping.is_none() {
                return_cars.push(Car {
                    uuid: car.uuid,
                    status: common_data::server::json::http::CarState::Offline,
                    name: car.name,
                })
            } else {
                let compare_time = car.last_ping.clone().unwrap();

                if compare_time >= offset {
                    return_cars.push(Car {
                        uuid: car.uuid,
                        status: common_data::server::json::http::CarState::Online,
                        name: car.name,
                    })
                } else {
                    return_cars.push(Car {
                        uuid: car.uuid,
                        status: common_data::server::json::http::CarState::Offline,
                        name: car.name,
                    })
                }
            }
        }

        return Ok(return_cars);
    }

    async fn fetch_car(&self, car_id: String) -> Result<Option<CarFull>, DatabaseError> {
        let car_query = sqlx::query!("SELECT * from cars where uuid = $1", car_id)
            .fetch_optional(&*self.pool)
            .await;

        if car_query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let car_opt = car_query.unwrap();

        if car_opt.is_none() {
            return Ok(None);
        }

        let car = car_opt.unwrap();

        return Ok(Some(CarFull {
            uuid: car.uuid,
            name: car.name,
            secret: car.secret,
            username: car.username,
            last_updated: car.last_updated,
            last_ping: car.last_ping,
        }));
    }

    async fn put_car(&self, car: CarFull) -> Result<(), DatabaseError> {
        let car_query = self.fetch_car(car.uuid.clone()).await;

        if car_query.is_err() {
            return Err(car_query.unwrap_err());
        }

        let car_opt = car_query.unwrap();

        if car_opt.is_some() {
            let query_status = sqlx::query!(
                "UPDATE cars SET secret = $2, name = $3, username = $4 WHERE uuid = $1",
                car.uuid,
                car.secret,
                car.name,
                car.username
            )
            .execute(&*self.pool)
            .await;

            if query_status.is_err() {
                return Err(DatabaseError::QueryError);
            }
        } else {
            let query_status = sqlx::query!(
                "INSERT INTO cars (uuid, secret, name, username) VALUES($1, $2, $3, $4)",
                car.uuid,
                car.secret,
                car.name,
                car.username
            )
            .execute(&*self.pool)
            .await;

            if query_status.is_err() {
                return Err(DatabaseError::QueryError);
            }
        }

        return Ok(());
    }

    async fn delete_car(&self, car_id: String) -> Result<(), DatabaseError> {
        let car_query = self.fetch_car(car_id.clone()).await;

        if car_query.is_err() {
            return Err(car_query.unwrap_err());
        }

        let car = car_query.unwrap();

        if car.is_none() {
            return Ok(());
        }

        let delet_query = sqlx::query!("DELETE FROM cars WHERE uuid = $1", car_id)
            .execute(&*self.pool)
            .await;

        if delet_query.is_err() {
            return Err(DatabaseError::QueryError);
        }
        return Ok(());
    }

    async fn delete_user_auth(&self, username: String) -> Result<(), DatabaseError> {
        let fetch_user = self.fetch_user_auth(username.clone()).await;

        if fetch_user.is_err() {
            return Err(fetch_user.unwrap_err());
        }

        let user = fetch_user.unwrap();

        if user.is_none() {
            return Ok(());
        }

        let query = sqlx::query!("DELETE FROM auth WHERE username = $1", username)
            .execute(&*self.pool)
            .await;

        if query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        return Ok(());
    }

    async fn ping_car_state(&self, car_id: String) -> Result<(), DatabaseError> {
        let query = sqlx::query!(
            "UPDATE cars SET last_ping = (NOW() at time zone 'utc') WHERE uuid = $1 ",
            car_id
        )
        .execute(&*self.pool)
        .await;

        if query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        return Ok(());
    }
}
