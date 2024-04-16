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

        let pg_pool = PgPool::connect(&connection_url).await;

        let pg_pool = match pg_pool {
            Ok(p) => p,
            Err(_err) => return Err(DatabaseError::ConnectionError),
        };

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
        let get_user = sqlx::query!("SELECT username from users where username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        let get_user = match get_user {
            Ok(p) => p,
            Err(_err) => return Err(DatabaseError::QueryError),
        };
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
        let auth_opt = sqlx::query!("SELECT * from auth WHERE username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        let auth_opt = match auth_opt {
            Ok(p) => p,
            Err(_err) => return Err(DatabaseError::QueryError),
        };

        match auth_opt {
            Some(auth) => Ok(Some(UserAuth {
                code: auth.code.clone(),
                created: auth.timestamp.clone(),
            })),
            None => Ok(None),
        }
    }

    async fn create_user_auth(&self, username: String, code: String) -> Result<(), DatabaseError> {
        let auth_fetch = self.fetch_user_auth(username.clone()).await;

        let auth_fetch = match auth_fetch {
            Ok(e) => e,
            Err(_) => return Err(DatabaseError::ServerError),
        };

        if auth_fetch.is_some() {
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

        match query {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ServerError),
        }
    }

    async fn fetch_user(&self, username: String) -> Result<Option<User>, DatabaseError> {
        let get_user = sqlx::query!("SELECT * from users where username = $1", username)
            .fetch_optional(&*self.pool)
            .await;

        let get_user = match get_user {
            Ok(e) => e,
            Err(_) => return Err(DatabaseError::ServerError),
        };

        match get_user {
            Some(user) => Ok(Some(User {
                username: user.username.clone(),
                last_login: user.lastsignin,
            })),
            None => Ok(None),
        }
    }

    async fn fetch_cars_by_user(&self, username: String) -> Result<Vec<Car>, DatabaseError> {
        let cars = sqlx::query!("SELECT * from cars where username = $1", username)
            .fetch_all(&*self.pool)
            .await;

        let cars = match cars {
            Ok(c) => c,
            Err(_) => return Err(DatabaseError::ServerError),
        };

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

        Ok(return_cars)
    }

    async fn fetch_car(&self, car_id: String) -> Result<Option<CarFull>, DatabaseError> {
        let car = sqlx::query!("SELECT * from cars where uuid = $1", car_id)
            .fetch_optional(&*self.pool)
            .await;
        let car = match car {
            Ok(c) => c,
            Err(_) => return Err(DatabaseError::ServerError),
        };

        match car {
            None => Ok(None),
            Some(c) => Ok(Some(CarFull {
                uuid: c.uuid,
                name: c.name,
                secret: c.secret,
                username: c.username,
                last_updated: c.last_updated,
                last_ping: c.last_ping,
            })),
        }
    }

    async fn put_car(&self, car: CarFull) -> Result<(), DatabaseError> {
        let car_opt = self.fetch_car(car.uuid.clone()).await;

        let car_opt = match car_opt {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

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
        let car_opt = self.fetch_car(car_id.clone()).await;

        let car_opt = match car_opt {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        if car_opt.is_none() {
            return Ok(());
        }

        let delete_query = sqlx::query!("DELETE FROM cars WHERE uuid = $1", car_id)
            .execute(&*self.pool)
            .await;

        match delete_query {
            Ok(_) => Ok(()),
            Err(_) => return Err(DatabaseError::QueryError),
        }
    }

    async fn delete_user_auth(&self, username: String) -> Result<(), DatabaseError> {
        let user = self.fetch_user_auth(username.clone()).await;

        let user = match user {
            Ok(u) => u,
            Err(e) => return Err(e),
        };

        if user.is_none() {
            return Ok(());
        }

        let query = sqlx::query!("DELETE FROM auth WHERE username = $1", username)
            .execute(&*self.pool)
            .await;

        match query {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ServerError),
        }
    }

    async fn ping_car_state(&self, car_id: String) -> Result<(), DatabaseError> {
        let query = sqlx::query!(
            "UPDATE cars SET last_ping = (NOW() at time zone 'utc') WHERE uuid = $1 ",
            car_id
        )
        .execute(&*self.pool)
        .await;

        match query {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::ServerError),
        }
    }
}
