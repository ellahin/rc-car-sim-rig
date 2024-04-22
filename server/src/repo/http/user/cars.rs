use crate::data::state::HttpState;
use crate::lib::auth;
use crate::repo::database::base::{CarFull, DataBase};

use common_data::server::json::http::{Car, CarState, CreateCar, CreateCarReturn, GetCars};

use actix_web::delete;
use actix_web::get;
use actix_web::put;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;

use serde_json;

use bcrypt;

use uuid::Uuid;

use rand::{distributions::Alphanumeric, Rng};

use chrono::prelude::*;

#[get("/user/cars")]
async fn get(state: Data<HttpState>, req: HttpRequest) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    let auth_state = match auth_token {
        None => return HttpResponse::Unauthorized().body("No authorization token"),
        Some(ah) => match ah.to_str() {
            Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
            Ok(ast) => match auth::validate_and_refresh(&ast, &state.jwt_secret) {
                Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
                Ok(a) => a,
            },
        },
    };

    let cars = state
        .database
        .fetch_cars_by_user(&auth_state.claims.email)
        .await;

    let cars = match cars {
        Ok(c) => c,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    let return_struct = GetCars { cars: cars };

    let return_string = match serde_json::to_string(&return_struct) {
        Ok(s) => s,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    match auth_state.refresh_token {
        Some(t) => HttpResponse::Ok()
            .insert_header(("Authorization", t))
            .body(return_string),
        None => return HttpResponse::Ok().body(return_string),
    }
}

#[put("/user/cars/")]
async fn add(state: Data<HttpState>, req: HttpRequest, data: Json<CreateCar>) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    let auth_state = match auth_token {
        None => return HttpResponse::Unauthorized().body("No authorization token"),
        Some(ah) => match ah.to_str() {
            Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
            Ok(ast) => match auth::validate_and_refresh(&ast, &state.jwt_secret) {
                Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
                Ok(a) => a,
            },
        },
    };

    let cars = state
        .database
        .fetch_cars_by_user(&auth_state.claims.email)
        .await;
    let cars = match cars {
        Ok(c) => c,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    if cars.len() >= 2 {
        return HttpResponse::Conflict().body("Only two cars are allowed, please delete a car");
    }

    drop(cars);

    if data.name.len() >= 251 {
        return HttpResponse::BadRequest().body("Car name cannot be more then 250 charictors");
    }

    let key_length = rand::thread_rng().gen_range(24..64);

    let key: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(key_length)
        .map(char::from)
        .collect();

    let key_becrypt = match bcrypt::hash(&key, bcrypt::DEFAULT_COST) {
        Ok(k) => k,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    let mut car_uuid = Uuid::new_v4().to_string();

    let car = state.database.fetch_car(&car_uuid).await;

    let mut car = match car {
        Ok(c) => c,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    while car.is_some() {
        car_uuid = Uuid::new_v4().to_string();

        let car_query = state.database.fetch_car(&car_uuid).await;

        car = match car_query {
            Ok(c) => c,
            Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
        };
    }

    let insert_query = state
        .database
        .put_car(&CarFull {
            name: data.name.clone(),
            uuid: car_uuid.clone(),
            secret: key_becrypt,
            username: auth_state.claims.email.clone(),
            last_ping: None,
            last_updated: Utc::now().naive_utc(),
        })
        .await;

    if insert_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let return_car = CreateCarReturn {
        name: data.name.clone(),
        uuid: car_uuid.clone(),
        api_key: key.clone(),
    };

    match serde_json::to_string(&return_car) {
        Ok(s) => HttpResponse::Ok().body(s),
        Err(_) => HttpResponse::ServiceUnavailable().body("Server Error"),
    }
}

#[delete("/user/cars/{car_id}")]
async fn remove(state: Data<HttpState>, req: HttpRequest, path: Path<(String,)>) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    let auth_state = match auth_token {
        None => return HttpResponse::Unauthorized().body("No authorization token"),
        Some(ah) => match ah.to_str() {
            Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
            Ok(ast) => match auth::validate_and_refresh(&ast, &state.jwt_secret) {
                Err(_) => return HttpResponse::BadRequest().body("Bad authorization token"),
                Ok(a) => a,
            },
        },
    };

    let car_uuid = path.into_inner().0;

    let car = state.database.fetch_car(&car_uuid).await;

    let car = match car {
        Ok(co) => match co {
            Some(c) => c,
            None => return HttpResponse::NotFound().body("Car does not exist"),
        },
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    if car.username != auth_state.claims.email {
        return HttpResponse::Unauthorized().body("Not Authorized");
    }

    let delet_query = state.database.delete_car(&car_uuid).await;

    if delet_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let cars = state
        .database
        .fetch_cars_by_user(&auth_state.claims.email)
        .await;
    let cars = match cars {
        Ok(c) => c,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    let return_struct = GetCars { cars: cars };

    let return_string = match serde_json::to_string(&return_struct) {
        Ok(s) => s,
        Err(_) => return HttpResponse::ServiceUnavailable().body("Server Error"),
    };

    match auth_state.refresh_token {
        None => HttpResponse::Ok().body(return_string),
        Some(t) => HttpResponse::Ok()
            .insert_header(("Authorization", t))
            .body(return_string),
    }
}
