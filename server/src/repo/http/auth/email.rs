use crate::data::state::HttpState;
use crate::repo::database::base::DataBase;

use common_data::server::data::jwt_claims::{AuthJwt, EmailAuthStartJwt};
use common_data::server::json::http::{AuthStartJson, AuthVerifyJson};

use actix_web::put;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpResponse;
use actix_web::Responder;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use email_address::*;

use rand::Rng;

use lettre::message::header::ContentType;
use lettre::Message;

use chrono::prelude::*;
use chrono::TimeDelta;

use std::str::FromStr;

#[put("/auth/email/")]
async fn put(state: Data<HttpState>, data: Json<AuthStartJson>) -> impl Responder {
    let email = match EmailAddress::from_str(&data.emailaddress) {
        Ok(e) => e,
        Err(_) => return HttpResponse::BadRequest().body("Bad Email"),
    };

    let mut auth_code_raw: Vec<char> = Vec::new();
    let mut rng = rand::thread_rng();
    for _ in 1..6 {
        let strings = format!("{}", rng.gen_range(0..9));

        auth_code_raw.push(strings.chars().last().unwrap());
    }

    let auth_code: String = auth_code_raw.iter().collect();

    let auth_insert = state
        .database
        .create_user_auth(&data.emailaddress, &auth_code)
        .await;

    if auth_insert.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let email_message = match Message::builder()
        .from(state.from_address.parse().unwrap())
        .to(email.to_string().parse().unwrap())
        .subject(format!("Your code is {}", auth_code))
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Hi, 

            Your Auth code to login is {}.

            If you did not request this login please ingore.",
            auth_code
        )) {
        Ok(e) => e,
        Err(_) => return HttpResponse::InternalServerError().body("Cannot send email"),
    };

    //let email_send = state.smtp_transport.send(&email_message);
    //
    //if email_send.is_err() {
    //   return HttpResponse::InternalServerError().body("Cannot send email");
    //}

    println!("Code is {}", auth_code);

    let current_time = Utc::now();
    // Offsetting by 15 min
    let offset_time = current_time + TimeDelta::try_seconds(900).unwrap();

    let jwt_claims: EmailAuthStartJwt = EmailAuthStartJwt {
        email: email.to_string(),
        iat: current_time.timestamp(),
        exp: offset_time.timestamp(),
    };

    match encode(
        &Header::default(),
        &jwt_claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(j) => HttpResponse::Ok().body(j),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

#[put("/auth/email/verify")]
pub async fn verify(state: Data<HttpState>, data: Json<AuthVerifyJson>) -> impl Responder {
    let validation = Validation::default();

    let token = match decode::<EmailAuthStartJwt>(
        &data.jwt,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    ) {
        Ok(t) => t.claims,
        Err(_) => return HttpResponse::Forbidden().body("Bad Auth Token"),
    };

    let auth = state.database.fetch_user_auth(&token.email).await;

    let auth = match auth {
        Err(_) => return HttpResponse::InternalServerError().body("Server Error"),
        Ok(ar) => match ar {
            Some(a) => a,
            None => return HttpResponse::Forbidden().body("Email Auth doesn't exist"),
        },
    };

    if auth.code != data.auth_code {
        return HttpResponse::Forbidden().body("Code Does not match");
    }

    let auth_remove = state.database.delete_user_auth(&token.email).await;

    if auth_remove.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let current_time = Utc::now();
    // Offsetting by 1 hour
    let offset_time = current_time + TimeDelta::try_seconds(3600).unwrap();

    let jwt_claims: AuthJwt = AuthJwt {
        email: token.email.clone(),
        signin_date: current_time.timestamp(),
        iat: current_time.timestamp(),
        exp: offset_time.timestamp(),
    };

    let jwt = match encode(
        &Header::default(),
        &jwt_claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(j) => j,
        Err(_) => return HttpResponse::InternalServerError().body("Server Error"),
    };

    match state.database.user_login(&token.email).await {
        Ok(_) => HttpResponse::Ok().body(jwt),
        Err(_) => HttpResponse::InternalServerError().body("Server Error"),
    }
}
