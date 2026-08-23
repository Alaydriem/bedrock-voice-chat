use std::sync::Arc;

use common::structs::certificate::CertificateFingerprint;
use entity::player;
use rocket::{
    State, async_trait,
    http::Status,
    mtls::Certificate,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::http::pool::Db;
use crate::services::{AuthService, CertificateRevocationService};

mod error;

pub(crate) use error::PlayerGuardError;

/// The authenticated player behind a client certificate.
///
/// Every player-facing route takes this instead of a bare `Certificate`, so resolving the
/// certificate to a player and checking it has not been revoked happen in one place rather
/// than being repeated at each route — and, as the channel routes showed, eventually not
/// repeated.
#[derive(Debug)]
pub struct PlayerGuard {
    pub player: player::Model,
}

#[async_trait]
impl<'r> FromRequest<'r> for PlayerGuard {
    type Error = PlayerGuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cert = match req.guard::<Certificate<'_>>().await {
            Outcome::Success(c) => c,
            Outcome::Error(_) | Outcome::Forward(_) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    PlayerGuardError::MissingCertificate,
                ));
            }
        };

        let db = match req.guard::<Db<'_>>().await {
            Outcome::Success(d) => d,
            Outcome::Error((s, _)) => return Outcome::Error((s, PlayerGuardError::Internal)),
            Outcome::Forward(s) => return Outcome::Forward(s),
        };
        let conn = db.into_inner();

        let revocations = match req
            .guard::<&State<Arc<CertificateRevocationService>>>()
            .await
        {
            Outcome::Success(r) => r,
            Outcome::Error((s, _)) => return Outcome::Error((s, PlayerGuardError::Internal)),
            Outcome::Forward(s) => return Outcome::Forward(s),
        };

        // `as_bytes` is the presented leaf DER, which is what the ban path fingerprints from
        // the stored PEM. Re-encoding from the parsed structure would not be byte-identical,
        // and the fingerprint would then never match.
        let fingerprint = CertificateFingerprint::from_der(cert.as_bytes());
        if revocations.is_revoked(conn, &fingerprint).await {
            tracing::warn!(%fingerprint, "PlayerGuard: refusing a revoked certificate");
            return Outcome::Error((Status::Forbidden, PlayerGuardError::Revoked));
        }

        let player = match AuthService::player_from_certificate(&cert, conn).await {
            Ok(player) => player,
            Err(s) if s == Status::Forbidden => {
                return Outcome::Error((Status::Forbidden, PlayerGuardError::PlayerNotFound));
            }
            Err(s) => return Outcome::Error((s, PlayerGuardError::Internal)),
        };

        // Enforced here rather than per route so it cannot be omitted by whichever route is
        // added next. Revocation covers the credential; this covers the person, and a ban
        // written before the revocation path existed is still only recorded on this flag.
        if player.banished {
            tracing::warn!(
                gamertag = %player.gamertag.clone().unwrap_or_default(),
                game = ?player.game,
                "PlayerGuard: refusing a banished player"
            );
            return Outcome::Error((Status::Forbidden, PlayerGuardError::Banished));
        }

        Outcome::Success(PlayerGuard { player })
    }
}

impl<'a> OpenApiFromRequest<'a> for PlayerGuard {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some("Player mTLS client certificate.".into()),
            data: okapi::openapi3::SecuritySchemeData::Http {
                scheme: "mutual".into(),
                bearer_format: None,
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("PlayerGuard".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "PlayerGuard".into(),
            security_scheme,
            security_req,
        ))
    }
}
