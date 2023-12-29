use rocket::{
    State,
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request}
};

use crate::config::ApplicationConfigServer;

/// Extracts the Access Token from the ncryptf request
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MCAccessToken(pub String);

#[derive(Debug)]
pub enum MCAccessTokenError {
    Invalid
}

#[async_trait]
impl<'r> FromRequest<'r> for MCAccessToken {
    type Error = MCAccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return match req.headers().get_one("X-MC-Access-Token") {
            Some(key) => {
                let at =  req.guard::<&State<ApplicationConfigServer>>().await.map(|config| MCAccessToken(config.minecraft.access_token.clone()));
                let current = Outcome::Success(MCAccessToken(key.to_string()));

                // Ensure that the access tokens match
                if at.eq(&current) {
                    Outcome::Success(MCAccessToken(key.to_string()))
                } else {
                    Outcome::Error((Status::Forbidden, MCAccessTokenError::Invalid))
                }
            },
            None => Outcome::Error((Status::BadRequest, MCAccessTokenError::Invalid)),
        };
    }
}