use crate::data::httpstate::HttpState;
use crate::data::userauthstruct::UserAuth;
use common_data::server::data::jwt_claims::EmailAuthStartJwt;
use common_data::server::json::http::AuthStartJson;

use actix_web::put;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpResponse;
use actix_web::Responder;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

use email_address::*;

use rand::Rng;

use lettre::message::header::ContentType;
use lettre::Message;
use lettre::SmtpTransport;
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
