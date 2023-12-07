use common::{
    rocket::{
        async_trait,
        http::Status,
        request::{FromRequest, Outcome, Request},
    },
};

/// Extracts the Access Token from the ncryptf request
pub struct MCAccessToken(pub String);

#[derive(Debug)]
pub enum MCAccessTokenError {
    Missing,
    Invalid,
}

#[async_trait]
impl<'r> FromRequest<'r> for MCAccessToken {
    type Error = MCAccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return match req.headers().get_one("X-MC-Access-Token") {
            Some(key) => Outcome::Success(MCAccessToken(key.to_string())),
            None => Outcome::Failure((Status::BadRequest, MCAccessTokenError::Missing)),
        };
    }
}