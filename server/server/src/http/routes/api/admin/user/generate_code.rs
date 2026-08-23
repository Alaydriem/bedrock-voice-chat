use common::curia;
use common::request::admin::GenerateCodeRequest;
use common::response::admin::GeneratedCodeResponse;
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::AuthCodeService;

/// Generate a one-time login code for an existing player.
///
/// Returns 404 if the player does not exist. Codes default to one hour and cap at 24.
#[openapi(tag = "Admin")]
#[post("/user/code", data = "<payload>")]
pub async fn generate_code(
    _admin: AdminGuard,
    db: Db<'_>,
    payload: Json<GenerateCodeRequest>,
) -> Result<Json<GeneratedCodeResponse>, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    if req.duration == 0 || req.duration > 86400 {
        return Err(Status::BadRequest);
    }

    crate::http::routes::api::admin::validate_gamertag(&req.gamertag)?;

    let player_record = player::Entity::find()
        .filter(player::Column::Gamertag.eq(req.gamertag.clone()))
        .filter(player::Column::Game.eq(req.game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            curia::error!("generate_code: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    let code = AuthCodeService::generate_code(conn, player_record.id, req.duration, req.ephemeral)
        .await
        .map_err(|e| {
            curia::error!("generate_code: insert failed: {}", e);
            Status::InternalServerError
        })?;

    Ok(Json(GeneratedCodeResponse {
        code,
        expires_in_seconds: req.duration,
    }))
}
