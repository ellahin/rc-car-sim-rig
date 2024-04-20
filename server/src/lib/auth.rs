use common_data::server::data::jwt_claims::AuthJwt;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use chrono::prelude::*;
use chrono::TimeDelta;

pub struct AuthState {
    pub claims: AuthJwt,
    pub refresh_token: Option<String>,
}

fn verify(token: String, secret: String) -> Result<AuthJwt, ()> {
    let validation = Validation::new(Algorithm::RS256);

    match decode::<AuthJwt>(
        &token,
        &DecodingKey::from_secret(secret.clone().as_bytes()),
        &validation,
    ) {
        Ok(t) => Ok(t.claims),
        Err(_) => Err(()),
    }
}

fn refresh_token(claims: AuthJwt, secret: String) -> Result<String, ()> {
    let current_time = Utc::now();
    // Offsetting by 15 min
    let offset_time = current_time.clone() + TimeDelta::try_seconds(900).unwrap();

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

    match encode(
        &jwt_header,
        &jwt_claims,
        &EncodingKey::from_secret(secret.clone().as_bytes()),
    ) {
        Ok(j) => Ok(j),
        Err(_) => Err(()),
    }
}

pub fn validate_and_refresh(token: String, secret: String) -> Result<AuthState, ()> {
    let token = verify(token, secret.clone());

    let token = match token {
        Ok(t) => t,
        Err(_) => return Err(()),
    };

    let now = Utc::now();

    let refresh_offset = now.clone() + TimeDelta::try_seconds(300).unwrap();

    if token.exp > now.timestamp() {
        return Err(());
    }

    if token.exp < refresh_offset.timestamp() {
        match refresh_token(token.clone(), secret.clone()) {
            Err(_) => return Err(()),
            Ok(a) => {
                return Ok(AuthState {
                    claims: token,
                    refresh_token: Some(a),
                })
            }
        };
    }

    return Ok(AuthState {
        claims: token,
        refresh_token: None,
    });
}
