use rocket::{
    async_trait,
    http::Status,
    mtls::Certificate,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use super::{GameAccessToken, PlayerGuard};

/// A reader of server-wide channel state: an authenticated player, or a game server
/// presenting its access token.
///
/// Either credential proves membership of this deployment, and neither is enough to
/// modify a channel — create, delete, rename and event keep `PlayerGuard`, because each
/// acts as a specific player and a game token names nobody.
pub struct ChannelReader;

#[async_trait]
impl<'r> FromRequest<'r> for ChannelReader {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // A presented certificate is judged by PlayerGuard alone. Falling through to the
        // token branch would let a revoked certificate or a banished player keep reading
        // by also sending a game token.
        if req.guard::<Certificate<'_>>().await.is_success() {
            return match req.guard::<PlayerGuard>().await {
                Outcome::Success(_) => Outcome::Success(ChannelReader),
                Outcome::Error((status, _)) => Outcome::Error((status, ())),
                Outcome::Forward(status) => Outcome::Forward(status),
            };
        }

        match req.guard::<GameAccessToken>().await {
            Outcome::Success(_) => Outcome::Success(ChannelReader),
            _ => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for ChannelReader {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some(
                "A player mTLS client certificate, or a game server access token as \
                 `Authorization: Bearer <token>`."
                    .into(),
            ),
            data: okapi::openapi3::SecuritySchemeData::Http {
                scheme: "bearer".into(),
                bearer_format: None,
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("ChannelReader".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "ChannelReader".into(),
            security_scheme,
            security_req,
        ))
    }
}
