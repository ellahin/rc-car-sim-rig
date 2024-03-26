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

#[get("/user/cars")]
async fn get(state: Data<HttpState>, req: HttpRequest) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    if auth_token.is_none() {
        return HttpResponse::Unauthorized().body("No authorization token");
    }

    let auth_state_raw = auth::validate_and_refresh(
        auth_token.unwrap().to_str().unwrap().to_string(),
        state.jwt_secret.clone(),
    );

    if auth_state_raw.is_err() {
        return HttpResponse::Unauthorized().body("Invalid Authorization");
    }

    let auth_state = auth_state_raw.unwrap();

    let car_query = state
        .database
        .fetch_cars_by_user(auth_state.claims.email.clone())
        .await;

    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let cars = car_query.unwrap();

    let mut return_cars: Vec<Car> = Vec::new();

    for car in cars {
        let car_temp = state
            .database
            .get_car_state(car.uuid.clone())
            .await
            .unwrap();

        if car_temp.is_none() {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Offline,
                name: car.name,
            })
        } else {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Online,
                name: car.name,
            })
        }
    }

    let return_struct = GetCars { cars: return_cars };

    if auth_state.refresh_token.is_some() {
        return HttpResponse::Ok()
            .insert_header(("Authorization", auth_state.refresh_token.unwrap()))
            .body(serde_json::to_string(&return_struct).unwrap());
    }

    return HttpResponse::Ok().body(serde_json::to_string(&return_struct).unwrap());
}

#[put("/user/cars/")]
async fn add(state: Data<HttpState>, req: HttpRequest, data: Json<CreateCar>) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    if auth_token.is_none() {
        return HttpResponse::Unauthorized().body("No authorization token");
    }

    let auth_state_raw = auth::validate_and_refresh(
        auth_token.unwrap().to_str().unwrap().to_string(),
        state.jwt_secret.clone(),
    );

    if auth_state_raw.is_err() {
        return HttpResponse::Unauthorized().body("Invalid Authorization");
    }

    let auth_state = auth_state_raw.unwrap();

    let car_query = state
        .database
        .fetch_cars_by_user(auth_state.claims.email.clone())
        .await;

    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let cars = car_query.unwrap();

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

    let key_becrypt = bcrypt::hash(key.clone(), bcrypt::DEFAULT_COST).unwrap();

    let mut car_uuid = Uuid::new_v4().to_string();

    let car_query = state.database.fetch_car(car_uuid.clone()).await;

    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let mut cars = car_query.unwrap();

    while cars.is_some() {
        car_uuid = Uuid::new_v4().to_string();

        let car_query = state.database.fetch_car(car_uuid.clone()).await;

        if car_query.is_err() {
            return HttpResponse::ServiceUnavailable().body("Server Error");
        }

        cars = car_query.unwrap();
    }

    let insert_query = state
        .database
        .put_car(CarFull {
            name: data.name.clone(),
            uuid: car_uuid.clone(),
            secret: key_becrypt,
            username: auth_state.claims.email.clone(),
            status: None,
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

    let return_string = serde_json::to_string(&return_car).unwrap();

    return HttpResponse::Ok().body(return_string);
}

#[delete("/user/cars/{car_id}")]
async fn remove(state: Data<HttpState>, req: HttpRequest, path: Path<(String,)>) -> impl Responder {
    let auth_token = req.headers().get("Authorization");

    if auth_token.is_none() {
        return HttpResponse::Unauthorized().body("No authorization token");
    }

    let auth_state_raw = auth::validate_and_refresh(
        auth_token.unwrap().to_str().unwrap().to_string(),
        state.jwt_secret.clone(),
    );

    if auth_state_raw.is_err() {
        return HttpResponse::Unauthorized().body("Invalid Authorization");
    }

    let auth_state = auth_state_raw.unwrap();

    let car_uuid = path.into_inner().0;

    let car_query = state.database.fetch_car(car_uuid.clone()).await;

    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let car_option = car_query.unwrap();

    if car_option.is_none() {
        HttpResponse::NotFound().body("Car does not exist");
    }

    let car = car_option.unwrap();

    if car.username != auth_state.claims.email {
        return HttpResponse::Unauthorized().body("Not Authorized");
    }

    let delet_query = state.database.delete_car(car_uuid.clone()).await;

    if delet_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let car_query = state
        .database
        .fetch_cars_by_user(auth_state.claims.email)
        .await;
    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let cars = car_query.unwrap();

    let mut return_cars: Vec<Car> = Vec::new();

    for car in cars {
        let car_temp = state
            .database
            .get_car_state(car.uuid.clone())
            .await
            .unwrap();

        if car_temp.is_none() {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Offline,
                name: car.name,
            })
        } else {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Online,
                name: car.name,
            })
        }
    }

    let return_struct = GetCars { cars: return_cars };

    if auth_state.refresh_token.is_some() {
        return HttpResponse::Ok()
            .insert_header(("Authorization", auth_state.refresh_token.unwrap()))
            .body(serde_json::to_string(&return_struct).unwrap());
    }

    return HttpResponse::Ok().body(serde_json::to_string(&return_struct).unwrap());
}
