use common::request::admin::SetPermissionRequest;
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::{PermissionService, PermissionServiceError};

#[openapi(tag = "Admin")]
#[put("/permission", data = "<payload>")]
pub async fn set_permission(
    _admin: AdminGuard,
    db: Db<'_>,
    payload: Json<SetPermissionRequest>,
) -> Result<Status, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    let player_record = player::Entity::find()
        .filter(player::Column::Gamertag.eq(req.gamertag.clone()))
        .filter(player::Column::Game.eq(req.game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            tracing::error!("set_permission: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    PermissionService::set_override(conn, player_record.id, &req.permission, req.effect)
        .await
        .map_err(|e| match e {
            PermissionServiceError::UnknownPermission(_) => Status::BadRequest,
            PermissionServiceError::Database(err) => {
                tracing::error!("set_permission: db error: {}", err);
                Status::InternalServerError
            }
        })?;

    Ok(Status::NoContent)
}
