use common_data::server::data::jwt_claims::AuthJwt;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use chrono::prelude::*;
use chrono::Duration;

pub struct AuthState {
    pub claims: AuthJwt,
    pub refresh_token: Option<String>,
}

fn verify(token: String, secret: String) -> Result<AuthJwt, ()> {
    let validation = Validation::new(Algorithm::RS256);

    let token_raw = decode::<AuthJwt>(
        &token,
        &DecodingKey::from_secret(secret.clone().as_bytes()),
        &validation,
    );

    if token_raw.is_err() {
        return Err(());
    }

    let token = token_raw.unwrap();

    return Ok(token.claims);
}

fn refresh_token(claims: AuthJwt, secret: String) -> Result<String, ()> {
    let current_time = Utc::now();
    // Offsetting by 15 min
    let offset_time = current_time.clone() + Duration::seconds(900);

    if current_time.timestamp() > claims.exp {
        return Err(());
    }

    let jwt_claims = AuthJwt {
        email: claims.email.clone(),
        signin_date: claims.signin_date.clone(),
        iat: current_time.timestamp(),
        exp: offset_time.timestamp(),
    };

    let jwt_header = Header::new(Algorithm::RS256);

    let jwt = encode(
        &jwt_header,
        &jwt_claims,
        &EncodingKey::from_secret(secret.clone().as_bytes()),
    );

    if jwt.is_err() {
        return Err(());
    }

    let token = jwt.unwrap();

    return Ok(token);
}

pub fn validate_and_refresh(token: String, secret: String) -> Result<AuthState, ()> {
    let token_raw = verify(token, secret.clone());

    if token_raw.is_err() {
        return Err(());
    }

    let token = token_raw.unwrap();

    let now = Utc::now();

    let refresh_offset = now.clone() + Duration::seconds(300);

    if token.exp > now.timestamp() {
        return Err(());
    }

    if token.exp < refresh_offset.timestamp() {
        let refresh = refresh_token(token.clone(), secret.clone());

        if refresh.is_err() {
            return Err(());
        }

        return Ok(AuthState {
            claims: token,
            refresh_token: Some(refresh.unwrap()),
        });
    }

    return Ok(AuthState {
        claims: token,
        refresh_token: None,
    });
}
