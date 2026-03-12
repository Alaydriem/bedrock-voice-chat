use common::{
    ncryptflib as ncryptf,
    request::CodeLoginRequest,
    structs::config::LoginResponse,
};
use rocket::{http::Status, serde::json::Json, State};

use sea_orm_rocket::Connection as SeaOrmConnection;

use crate::config::{ApplicationConfigFeatures, ApplicationConfigServer};
use crate::rs::dtos::ncryptf::JsonMessage;
use crate::rs::pool::AppDb;
use crate::services::{AuthCodeError, AuthCodeService, AuthError, AuthService};

#[post("/auth/code", data = "<payload>")]
pub async fn code_authenticate(
    db: SeaOrmConnection<'_, AppDb>,
    payload: Json<CodeLoginRequest>,
    config: &State<ApplicationConfigServer>,
    features: &State<ApplicationConfigFeatures>,
) -> ncryptf::rocket::JsonResponse<JsonMessage<LoginResponse>> {
    let conn = db.into_inner();

    if !features.code_login {
        return JsonMessage::create(Status::NotFound, None, None, None);
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
                    JsonMessage::create(Status::NotFound, None, None, None)
                }
                AuthCodeError::GamertagMismatch | AuthCodeError::CodeAlreadyUsed => {
                    JsonMessage::create(Status::Forbidden, None, None, None)
                }
                AuthCodeError::CodeExpired => {
                    JsonMessage::create(Status::Gone, None, None, None)
                }
                _ => JsonMessage::create(Status::InternalServerError, None, None, None),
            };
        }
    };

    // Build login response using AuthService
    match AuthService::build_login_response(
        conn,
        config.inner(),
        player_record.gamertag.unwrap_or_default(),
        player_record.gamerpic.unwrap_or_default(),
        player_record.game,
    )
    .await
    {
        Ok(response) => JsonMessage::create(Status::Ok, Some(response), None, None),
        Err(e) => {
            tracing::error!("Code login - build response failed: {}", e);
            match e {
                AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                    JsonMessage::create(Status::Forbidden, None, None, None)
                }
                _ => JsonMessage::create(Status::InternalServerError, None, None, None),
            }
        }
    }
}
