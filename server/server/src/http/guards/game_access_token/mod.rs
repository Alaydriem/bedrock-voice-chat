use std::sync::Arc;

use rocket::{
    State, async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::services::AccessTokenService;

mod error;

pub(crate) use error::GameAccessTokenError;

const BEARER_PREFIX: &str = "Bearer ";

/// A game server's access token, carried as `Authorization: Bearer <token>`.
///
/// Two forms are accepted. An identified token, `bvc_<id>_<secret>`, is looked up by id and
/// compared against a stored hash; it can be revoked on its own. The pre-identifier scalar
/// is compared whole, and exists so a deployment that predates identified tokens keeps
/// working until its mods are moved.
///
/// `X-MC-Access-Token` is **not** accepted. This is a forced upgrade: a server on this version
/// requires a mod on this version.
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

        let service = match req.guard::<&State<Arc<AccessTokenService>>>().await {
            Outcome::Success(service) => service,
            _ => {
                return Outcome::Error((
                    Status::InternalServerError,
                    GameAccessTokenError::Invalid,
                ));
            }
        };

        if service.verify(&presented) {
            Outcome::Success(GameAccessToken(presented))
        } else if service.has_no_credential() {
            Outcome::Error((Status::Forbidden, GameAccessTokenError::NotConfigured))
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
