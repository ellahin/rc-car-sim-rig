use crate::data::httpstate::HttpState;
use crate::data::userauthstruct::UserAuth;
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
use chrono::Duration;

use std::str::FromStr;
use std::time::SystemTime;

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

    let auth_struct = UserAuth {
        code: auth_code.clone(),
        created: SystemTime::now(),
    };

    let mut authstate = state.user_auth.lock().unwrap();

    authstate.insert(data.emailaddress.clone(), auth_struct);

    drop(authstate);

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
    let offset_time = current_time.clone() + Duration::seconds(900);

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
    let mut validation = Validation::new(Algorithm::RS256);

    let token_raw = decode::<EmailAuthStartJwt>(
        &data.jwt,
        &DecodingKey::from_secret(state.jwt_secret.clone().as_bytes()),
        &validation,
    );

    if token_raw.is_err() {
        return HttpResponse::Forbidden().body("Bad Auth Token");
    }

    let token = token_raw.unwrap().claims;

    let mut authstate = state.user_auth.lock().unwrap();

    let auth_raw = authstate.get(&token.email.clone());

    if auth_raw.is_none() {
        return HttpResponse::Forbidden().body("Email Auth doesn't exist");
    }

    let auth = auth_raw.unwrap();

    if auth.code != data.auth_code {
        return HttpResponse::Forbidden().body("Code Does not match");
    }

    authstate.remove(&token.email.clone());

    drop(authstate);

    let current_time = Utc::now();
    // Offsetting by 1 hour
    let offset_time = current_time.clone() + Duration::seconds(3600);

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

    let get_user_raw = sqlx::query!(
        "SELECT username from users where username = ?1",
        token.email
    )
    .fetch_optional(&state.sqlx)
    .await;

    if get_user_raw.is_err() {
        return HttpResponse::InternalServerError().body("Server Error");
    }

    let get_user = get_user_raw.unwrap();

    if get_user.is_none() {
        let _ = sqlx::query!(
            "INSERT INTO users(username, lastsignin) VALUES(?1, datetime('now'))",
            token.email
        )
        .execute(&state.sqlx)
        .await;
    } else {
        let _ = sqlx::query!(
            "update users set lastsignin = datetime('now') where username = ?1",
            token.email
        )
        .execute(&state.sqlx)
        .await;
    }

    return HttpResponse::Ok().body(jwt.unwrap());
}
