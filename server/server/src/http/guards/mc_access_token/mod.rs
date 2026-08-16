use rocket::{
    State, async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::config::Server;

mod error;

pub(crate) use error::MCAccessTokenError;

/// Extracts the Access Token from the ncryptf request
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MCAccessToken(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for MCAccessToken {
    type Error = MCAccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return match req.headers().get_one("X-MC-Access-Token") {
            Some(key) => {
                let expected = match req.guard::<&State<Server>>().await {
                    Outcome::Success(config) => config.minecraft.access_token.clone(),
                    _ => {
                        return Outcome::Error((
                            Status::InternalServerError,
                            MCAccessTokenError::Invalid,
                        ));
                    }
                };

                // Constant-time compare so a network attacker cannot recover the
                // token byte-by-byte from response-time differences.
                if constant_time_eq::constant_time_eq(expected.as_bytes(), key.as_bytes()) {
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
