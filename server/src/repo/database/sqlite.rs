use crate::repo::database::base::{CarFull, CarState, DataBase, DatabaseError, User, UserAuth};

use common_data::server::json::http::Car;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time;

use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::sqlite::Sqlite;
use sqlx::Pool;
use sqlx::SqlitePool;

use chrono::prelude::*;
use chrono::TimeDelta;

use tokio;

use async_trait::async_trait;

#[derive(Clone)]
pub struct SqliteDatabase {
    car_state: Arc<Mutex<HashMap<String, CarState>>>,
    user_auth: Arc<Mutex<HashMap<String, UserAuth>>>,
    pool: Pool<Sqlite>,
}

impl DataBase for SqliteDatabase {
    async fn new(connection_url: String) -> Result<Box<Self>, DatabaseError> {
        if !sqlx::Sqlite::database_exists(&connection_url)
            .await
            .unwrap()
        {
            sqlx::Sqlite::create_database(&connection_url)
                .await
                .unwrap();
        }

        let migration_path = Path::new("./migrations");

        let sql_pool = SqlitePool::connect(&connection_url).await.unwrap();

        Migrator::new(migration_path)
            .await
            .unwrap()
            .run(&sql_pool)
            .await
            .unwrap();

        let cars_raw: HashMap<String, CarState> = HashMap::new();
        let cars = Arc::new(Mutex::new(cars_raw));

        let cars_cleanup = Arc::clone(&cars);
        tokio::spawn(async move {
            loop {
                let now = Utc::now();
                let offset = now - TimeDelta::try_seconds(30).unwrap();
                let mut cars_lock = cars_cleanup.lock().unwrap();
                let mut to_delete: Vec<String> = Vec::new();

                for (id, car) in cars_lock.iter().clone() {
                    if car.last_changed <= offset.timestamp() {
                        to_delete.push(id.clone());
                    }
                }

                for delete in to_delete.iter() {
                    let _ = cars_lock.remove(delete);
                }
                let _ = tokio::time::sleep(time::Duration::from_secs(30));
            }
        });

        let userauth_raw: HashMap<String, UserAuth> = HashMap::new();
        let userauth = Arc::new(Mutex::new(userauth_raw));

        let userauth_cleanup = Arc::clone(&userauth);

        tokio::spawn(async move {
            loop {
                let now = Utc::now();
                let offset = now - TimeDelta::try_seconds(900).unwrap();
                let mut userauth_lock = userauth_cleanup.lock().unwrap();
                let mut to_delete: Vec<String> = Vec::new();

                for (id, user) in userauth_lock.iter().clone() {
                    if user.created <= offset {
                        to_delete.push(id.clone());
                    }
                }

                for delete in to_delete.iter() {
                    let _ = userauth_lock.remove(delete);
                }
                let _ = tokio::time::sleep(time::Duration::from_secs(30));
            }
        });

        return Ok(Box::new(SqliteDatabase {
            car_state: cars,
            user_auth: userauth,
            pool: sql_pool,
        }));
    }
    async fn user_login(&self, username: String) -> Result<(), DatabaseError> {
        let get_user_raw = sqlx::query!("SELECT username from users where username = ?1", username)
            .fetch_optional(&self.pool)
            .await;

        if get_user_raw.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let get_user = get_user_raw.unwrap();

        let login_time = Utc::now().timestamp();

        if get_user.is_none() {
            let _ = sqlx::query!(
                "INSERT INTO users(username, lastsignin) VALUES(?1, ?2)",
                username,
                login_time
            )
            .execute(&self.pool)
            .await;
        } else {
            let _ = sqlx::query!(
                "update users set lastsignin = ?2 where username = ?1",
                username,
                login_time
            )
            .execute(&self.pool)
            .await;
        }

        return Ok(());
    }

    async fn fetch_user_auth(&self, username: String) -> Result<Option<UserAuth>, DatabaseError> {
        let authstate = self.user_auth.lock().unwrap();

        let res_state = authstate.get(&username);

        if res_state.is_some() {
            let temp = res_state.unwrap().clone();
            return Ok(Some(temp));
        }
        return Ok(None);
    }

    async fn create_user_auth(&self, username: String, code: String) -> Result<(), DatabaseError> {
        let mut authstate = self.user_auth.lock().unwrap();

        let auth_struct = UserAuth {
            code: code.clone(),
            created: Utc::now(),
        };

        authstate.insert(username, auth_struct);

        return Ok(());
    }

    async fn fetch_user(&self, username: String) -> Result<Option<User>, DatabaseError> {
        let get_user_raw = sqlx::query!("SELECT * from users where username = ?1", username)
            .fetch_optional(&self.pool)
            .await;

        if get_user_raw.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let get_user = get_user_raw.unwrap();

        if get_user.is_some() {
            let temp = get_user.unwrap();
            let object = User {
                username: temp.username.clone(),
                last_login: Utc.timestamp_nanos(temp.lastsignin.clone()),
            };

            return Ok(Some(object));
        }

        return Ok(None);
    }

    async fn fetch_cars_by_user(&self, username: String) -> Result<Vec<Car>, DatabaseError> {
        let car_query = sqlx::query!("SELECT * from cars where username = ?1", username)
            .fetch_all(&self.pool)
            .await;

        if car_query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let cars = car_query.unwrap();

        let mut return_cars: Vec<Car> = Vec::new();

        let car_state = self.car_state.lock().unwrap();

        for car in cars {
            let car_temp = car_state.get(&car.uuid);

            if car_temp.is_none() {
                return_cars.push(Car {
                    uuid: car.uuid,
                    status: common_data::server::json::http::CarState::Offline,
                    name: car.name,
                })
            } else {
                return_cars.push(Car {
                    uuid: car.uuid,
                    status: common_data::server::json::http::CarState::Online,
                    name: car.name,
                })
            }
        }

        return Ok(return_cars);
    }

    async fn fetch_car(&self, car_id: String) -> Result<Option<CarFull>, DatabaseError> {
        let car_query = sqlx::query!("SELECT * from cars where uuid = ?1", car_id)
            .fetch_optional(&self.pool)
            .await;

        if car_query.is_err() {
            return Err(DatabaseError::ServerError);
        }

        let car = car_query.unwrap();

        if car.is_none() {
            return Ok(None);
        }

        let car_state = self.car_state.lock().unwrap();

        let car_status = car_state.get(&car_id);

        if car_status.is_none() {
            let temp = car.unwrap();
            return Ok(Some(CarFull {
                uuid: temp.uuid,
                status: None,
                name: temp.name,
                secret: temp.secret,
                username: temp.username,
            }));
        } else {
            let temp = car_status.unwrap().clone();
            let car_temp = car.unwrap();
            return Ok(Some(CarFull {
                uuid: car_temp.uuid,
                status: Some(temp),
                name: car_temp.name,
                secret: car_temp.secret,
                username: car_temp.username,
            }));
        }
    }

    async fn put_car_state(
        &self,
        car_id: String,
        car_state: CarState,
    ) -> Result<(), DatabaseError> {
        let mut car_hash = self.car_state.lock().unwrap();

        car_hash.insert(car_id, car_state);

        return Ok(());
    }

    async fn put_car(&self, car: CarFull) -> Result<(), DatabaseError> {
        let car_query = self.fetch_car(car.uuid.clone()).await;

        if car_query.is_err() {
            return Err(car_query.unwrap_err());
        }

        let car_opt = car_query.unwrap();

        if car_opt.is_some() {
            let query_status = sqlx::query!(
                "UPDATE cars SET secret = ?2, name = ?3, username = ?4 WHERE uuid = ?1",
                car.uuid,
                car.secret,
                car.name,
                car.username
            )
            .execute(&self.pool)
            .await;

            if query_status.is_err() {
                return Err(DatabaseError::QueryError);
            }
        } else {
            let query_status = sqlx::query!(
                "INSERT INTO cars (uuid, secret, name, username) VALUES(?1, ?2, ?3, ?4)",
                car.uuid,
                car.secret,
                car.name,
                car.username
            )
            .execute(&self.pool)
            .await;

            if query_status.is_err() {
                return Err(DatabaseError::QueryError);
            }
        }

        return Ok(());
    }

    async fn get_car_state(&self, car_id: String) -> Result<Option<CarState>, DatabaseError> {
        let car_state = self.car_state.lock().unwrap();

        let car = car_state.get(&car_id);

        if car.is_none() {
            return Ok(None);
        }

        return Ok(Some(car.unwrap().clone()));
    }

    async fn ping_car_state(&self, car_id: String) -> Result<(), DatabaseError> {
        let mut car_state = self.car_state.lock().unwrap();

        let car = &car_state.get(&car_id);

        if car.is_none() {
            return Err(DatabaseError::DoesNotExist);
        }

        let mut car_obj = car.unwrap().clone();

        car_obj.last_changed = Utc::now().timestamp();

        car_state.insert(car_id.clone(), car_obj.clone());

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

        let delet_query = sqlx::query!("DELETE FROM cars WHERE uuid = ?1", car_id)
            .execute(&self.pool)
            .await;

        if delet_query.is_err() {
            return Err(DatabaseError::QueryError);
        }
        return Ok(());
    }

    async fn delete_user_auth(&self, username: String) -> Result<(), DatabaseError> {
        let user = self.fetch_user_auth(username.clone()).await;

        if user.is_err() {
            return Err(user.unwrap_err());
        }

        if user.unwrap().is_none() {
            return Ok(());
        }

        let mut user_auth = self.user_auth.lock().unwrap();

        user_auth.remove(&username.clone());

        return Ok(());
    }
}
