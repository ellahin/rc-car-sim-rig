use crate::data::httpstate::HttpState;
use crate::lib::auth;

use common_data::server::json::http::{Car, CarState, GetCars};

use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;

use sqlx;

use serde_json;

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

    let car_query = sqlx::query!(
        "SELECT * from cars where username = ?1",
        auth_state.claims.email
    )
    .fetch_all(&state.sqlx)
    .await;

    if car_query.is_err() {
        return HttpResponse::ServiceUnavailable().body("Server Error");
    }

    let cars = car_query.unwrap();

    let mut return_cars: Vec<Car> = Vec::new();

    let car_state = state.cars.lock().unwrap();

    for car in cars {
        let car_temp = car_state.get(&car.uuid);

        if car_temp.is_none() {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Offline,
            })
        } else {
            return_cars.push(Car {
                uuid: car.uuid,
                status: CarState::Online,
            })
        }
    }

    drop(car_state);

    let return_struct = GetCars { cars: return_cars };

    if auth_state.refresh_token.is_some() {
        return HttpResponse::Ok()
            .insert_header(("Authorization", auth_state.refresh_token.unwrap()))
            .body(serde_json::to_string(&return_struct).unwrap());
    }

    return HttpResponse::Ok().body(serde_json::to_string(&return_struct).unwrap());
}
