use common::{
    request::CodeLoginRequest,
    response::LoginResponse,
};
use rocket::{http::Status, serde::json::Json, State};
use rocket_okapi::openapi;

use crate::config::{Features, Permissions, Server};
use crate::http::dtos::ncryptf::JsonMessage;
use crate::http::openapi::NcryptfJsonResponse;
use crate::http::pool::Db;
use crate::services::{AuthCodeError, AuthCodeService, AuthError, AuthService, PermissionService};

#[openapi(tag = "Authentication")]
#[post("/auth/code", data = "<payload>")]
pub async fn code_authenticate(
    db: Db<'_>,
    payload: Json<CodeLoginRequest>,
    config: &State<Server>,
    features: &State<Features>,
    perm_config: &State<Permissions>,
) -> NcryptfJsonResponse<LoginResponse> {
    let conn = db.into_inner();

    if !features.code_login {
        return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::NotFound, None, None, None));
    }

    let code = &payload.0.code;
    let gamertag = &payload.0.gamertag;

    // Validate the code and get the associated player
    let player_record = match AuthCodeService::validate_and_consume_code(conn, code, gamertag).await
    {
        Ok(player) => player,
        Err(e) => {
            tracing::error!("Code login failed: {}", e);
            return match e {
                AuthCodeError::CodeNotFound => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::NotFound, None, None, None))
                }
                AuthCodeError::GamertagMismatch | AuthCodeError::CodeAlreadyUsed => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None))
                }
                AuthCodeError::CodeExpired => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Gone, None, None, None))
                }
                _ => NcryptfJsonResponse::from_inner(JsonMessage::create(Status::InternalServerError, None, None, None)),
            };
        }
    };

    // Build login response using AuthService
    let perm_service = PermissionService::new(perm_config.defaults.clone());
    match AuthService::build_login_response(
        conn,
        config.inner(),
        Some(&perm_service),
        player_record.gamertag.unwrap_or_default(),
        player_record.gamerpic.unwrap_or_default(),
        player_record.game,
    )
    .await
    {
        Ok(response) => NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None)),
        Err(e) => {
            tracing::error!("Code login - build response failed: {}", e);
            match e {
                AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None))
                }
                _ => NcryptfJsonResponse::from_inner(JsonMessage::create(Status::InternalServerError, None, None, None)),
            }
        }
    }
}
