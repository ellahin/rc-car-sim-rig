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
use lettre::Transport;

use chrono::prelude::*;
use chrono::TimeDelta;

use std::str::FromStr;

#[put("/auth/email/")]
async fn put(state: Data<HttpState>, data: Json<AuthStartJson>) -> impl Responder {
    let email_res = EmailAddress::from_str(&data.emailaddress.clone());

    if email_res.is_err() {
        return HttpResponse::BadRequest().body("Bad Email");
    }

    let email = email_res.unwrap();

    let mut auth_code_raw: Vec<char> = Vec::new();
    let mut rng = rand::thread_rng();
    for _ in 1..6 {
        let strings = format!("{}", rng.gen_range(0..9));

        auth_code_raw.push(strings.chars().last().unwrap());
    }

    let auth_code: String = auth_code_raw.iter().collect();

    let auth_insert = state
        .database
        .create_user_auth(data.emailaddress.clone(), auth_code.clone())
        .await;

    if auth_insert.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let email_message = Message::builder()
        .from(state.from_address.clone().parse().unwrap())
        .to(email.to_string().parse().unwrap())
        .subject(format!("Your code is {}", auth_code.clone()))
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Hi, 

            Your Auth code to login is {}.

            If you did not request this login please ingore.",
            auth_code
        ))
        .unwrap();

    let email_send = state.smtp_transport.send(&email_message);

    if email_send.is_err() {
        return HttpResponse::InternalServerError().body("Cannot send email");
    }

    let current_time = Utc::now();
    // Offsetting by 15 min
    let offset_time = current_time.clone() + TimeDelta::try_seconds(900).unwrap();

    let jwt_claims: EmailAuthStartJwt = EmailAuthStartJwt {
        email: email.to_string(),
        iat: current_time.timestamp(),
        exp: offset_time.timestamp(),
    };

    let jwt_header = Header::new(Algorithm::RS256);

    let jwt = encode(
        &jwt_header,
        &jwt_claims,
        &EncodingKey::from_secret(state.jwt_secret.clone().as_bytes()),
    );

    if jwt.is_err() {
        println!("Cannot encode JWT Toket: {}", jwt.unwrap_err());
        return HttpResponse::InternalServerError().body("Server Error");
    }

    return HttpResponse::Ok().body(jwt.unwrap());
}

#[put("/auth/email/verify")]
pub async fn verify(state: Data<HttpState>, data: Json<AuthVerifyJson>) -> impl Responder {
    let validation = Validation::new(Algorithm::RS256);

    let token_raw = decode::<EmailAuthStartJwt>(
        &data.jwt,
        &DecodingKey::from_secret(state.jwt_secret.clone().as_bytes()),
        &validation,
    );

    if token_raw.is_err() {
        return HttpResponse::Forbidden().body("Bad Auth Token");
    }

    let token = token_raw.unwrap().claims;

    let auth_err = state.database.fetch_user_auth(token.email.clone()).await;

    if auth_err.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let auth_raw = auth_err.unwrap();

    if auth_raw.is_none() {
        return HttpResponse::Forbidden().body("Email Auth doesn't exist");
    }

    let auth = auth_raw.unwrap();

    if auth.code != data.auth_code {
        return HttpResponse::Forbidden().body("Code Does not match");
    }

    let auth_remove = state.database.delete_user_auth(token.email.clone()).await;

    if auth_remove.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let current_time = Utc::now();
    // Offsetting by 1 hour
    let offset_time = current_time.clone() + TimeDelta::try_seconds(3600).unwrap();

    let jwt_claims: AuthJwt = AuthJwt {
        email: token.email.clone(),
        signin_date: current_time.timestamp(),
        iat: current_time.timestamp(),
        exp: offset_time.timestamp(),
    };

    let jwt_header = Header::new(Algorithm::RS256);

    let jwt = encode(
        &jwt_header,
        &jwt_claims,
        &EncodingKey::from_secret(state.jwt_secret.clone().as_bytes()),
    );

    if jwt.is_err() {
        println!("Cannot encode JWT Toket: {}", jwt.unwrap_err());
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let user_update = state.database.user_login(token.email).await;

    if user_update.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    return HttpResponse::Ok().body(jwt.unwrap());
}
