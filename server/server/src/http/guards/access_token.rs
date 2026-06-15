use common::ncryptflib as ncryptf;

use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};

use crate::http::guards::AccessTokenError;

/// Extracts the Access Token from the ncryptf request
#[allow(dead_code)]
pub struct AccessToken(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for AccessToken {
    type Error = AccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return match req.headers().get_one("Authorization") {
            Some(key) => {
                match ncryptf::Authorization::extract_params_from_header_string(key.to_string()) {
                    Ok(params) => Outcome::Success(AccessToken(params.access_token)),
                    Err(_) => Outcome::Error((Status::BadRequest, AccessTokenError::Invalid)),
                }
            }
            None => Outcome::Error((Status::BadRequest, AccessTokenError::Missing)),
        };
    }
}
