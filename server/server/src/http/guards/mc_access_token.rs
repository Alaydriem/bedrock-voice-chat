use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
    State,
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::config::Server;

/// Extracts the Access Token from the ncryptf request
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MCAccessToken(pub String);

#[derive(Debug)]
pub enum MCAccessTokenError {
    Invalid,
}

#[async_trait]
impl<'r> FromRequest<'r> for MCAccessToken {
    type Error = MCAccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return match req.headers().get_one("X-MC-Access-Token") {
            Some(key) => {
                let at = req
                    .guard::<&State<Server>>()
                    .await
                    .map(|config| MCAccessToken(config.minecraft.access_token.clone()));
                let current = Outcome::Success(MCAccessToken(key.to_string()));

                // Ensure that the access tokens match
                if at.eq(&current) {
                    Outcome::Success(MCAccessToken(key.to_string()))
                } else {
                    Outcome::Error((Status::Forbidden, MCAccessTokenError::Invalid))
                }
            }
            None => Outcome::Error((Status::BadRequest, MCAccessTokenError::Invalid)),
        };
    }
}

impl<'a> OpenApiFromRequest<'a> for MCAccessToken {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some("Minecraft mod access token".into()),
            data: okapi::openapi3::SecuritySchemeData::ApiKey {
                name: "X-MC-Access-Token".into(),
                location: "header".into(),
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("MCAccessToken".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "MCAccessToken".into(),
            security_scheme,
            security_req,
        ))
    }
}
