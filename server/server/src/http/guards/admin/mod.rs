use common::curia;
use common::structs::permission::Permission;
use entity::player;
use rocket::{
    State, async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use crate::config::Permissions;
use crate::http::guards::PlayerGuard;
use crate::http::pool::Db;
use crate::services::PermissionService;

mod error;

pub(crate) use error::AdminGuardError;

#[derive(Debug)]
pub struct AdminGuard {
    pub player: player::Model,
}

#[async_trait]
impl<'r> FromRequest<'r> for AdminGuard {
    type Error = AdminGuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Built on PlayerGuard rather than repeating its work, so certificate resolution and
        // the revocation check happen in exactly one place.
        let player = match req.guard::<PlayerGuard>().await {
            Outcome::Success(guard) => guard.player,
            Outcome::Error((s, _)) if s == Status::Unauthorized => {
                return Outcome::Error((Status::Unauthorized, AdminGuardError::MissingCertificate));
            }
            Outcome::Error((s, _)) => {
                return Outcome::Error((s, AdminGuardError::PlayerNotFound));
            }
            Outcome::Forward(s) => return Outcome::Forward(s),
        };

        let db = match req.guard::<Db<'_>>().await {
            Outcome::Success(d) => d,
            Outcome::Error((s, _)) => return Outcome::Error((s, AdminGuardError::Internal)),
            Outcome::Forward(s) => return Outcome::Forward(s),
        };

        let conn = db.into_inner();

        // No banished check here: `PlayerGuard` already refused one, so reaching this line
        // means the caller is not banished.
        let perm_config = match req.guard::<&State<Permissions>>().await {
            Outcome::Success(p) => p,
            Outcome::Error((s, _)) => return Outcome::Error((s, AdminGuardError::Internal)),
            Outcome::Forward(s) => return Outcome::Forward(s),
        };

        let perm_service = PermissionService::new(perm_config.defaults.clone());
        let allowed = perm_service
            .evaluate(conn, player.id, &Permission::Admin)
            .await;

        if allowed {
            Outcome::Success(AdminGuard { player })
        } else {
            curia::warn!(
                "AdminGuard: player {} ({:?}) is missing 'admin' permission",
                player.gamertag.clone().unwrap_or_default(),
                player.game,
            );
            Outcome::Error((Status::Forbidden, AdminGuardError::Forbidden))
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for AdminGuard {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = okapi::openapi3::SecurityScheme {
            description: Some(
                "Operator mTLS client certificate. Caller must additionally hold the `admin` permission."
                    .into(),
            ),
            data: okapi::openapi3::SecuritySchemeData::Http {
                scheme: "mutual".into(),
                bearer_format: None,
            },
            extensions: Default::default(),
        };
        let mut security_req = okapi::openapi3::SecurityRequirement::new();
        security_req.insert("AdminGuard".into(), Vec::new());
        Ok(RequestHeaderInput::Security(
            "AdminGuard".into(),
            security_scheme,
            security_req,
        ))
    }
}
