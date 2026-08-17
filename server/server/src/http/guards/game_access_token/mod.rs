use rocket::{
    State, async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::config::Server;

mod error;

pub(crate) use error::GameAccessTokenError;

const BEARER_PREFIX: &str = "Bearer ";

/// The game server's shared access token, carried as `Authorization: Bearer <token>`.
///
/// `X-MC-Access-Token` is **not** accepted. This is a forced upgrade: a server on this version
/// requires a mod on this version. The token's value, entropy and storage are unchanged, so
/// nothing is reconfigured — the mod is rebuilt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameAccessToken(pub String);

impl GameAccessToken {
    // The `Bearer ` prefix is required rather than assumed. ncryptf uses this same header on
    // `/api/auth/*` with a different scheme, and accepting any value would take its HMAC
    // parameter string as a credential the moment a route carried both guards.
    fn bearer_of(header: &str) -> Option<&str> {
        header.strip_prefix(BEARER_PREFIX).map(str::trim)
    }

    fn presented(req: &Request<'_>) -> Option<String> {
        req.headers()
            .get_one("Authorization")
            .and_then(Self::bearer_of)
            .map(str::to_string)
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for GameAccessToken {
    type Error = GameAccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(presented) = Self::presented(req) else {
            return Outcome::Error((Status::BadRequest, GameAccessTokenError::Missing));
        };

        let expected = match req.guard::<&State<Server>>().await {
            Outcome::Success(config) => config.minecraft.access_token.clone(),
            _ => {
                return Outcome::Error((
                    Status::InternalServerError,
                    GameAccessTokenError::Invalid,
                ));
            }
        };

        // Constant-time so a network attacker cannot recover the token byte-by-byte from
        // response-time differences. Unchanged from the header this replaces.
        if constant_time_eq::constant_time_eq(expected.as_bytes(), presented.as_bytes()) {
            Outcome::Success(GameAccessToken(presented))
        } else {
            Outcome::Error((Status::Forbidden, GameAccessTokenError::Invalid))
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for GameAccessToken {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some(
                "Game server access token, as `Authorization: Bearer <token>`.".into(),
            ),
            data: okapi::openapi3::SecuritySchemeData::Http {
                scheme: "bearer".into(),
                bearer_format: None,
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("GameAccessToken".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "GameAccessToken".into(),
            security_scheme,
            security_req,
        ))
    }
}
