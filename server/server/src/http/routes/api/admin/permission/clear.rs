use common::request::admin::ClearPermissionRequest;
use common::structs::permission::Permission;
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::PermissionService;

#[openapi(tag = "Admin")]
#[delete("/permission", data = "<payload>")]
pub async fn clear_permission(
    admin: AdminGuard,
    db: Db<'_>,
    payload: Json<ClearPermissionRequest>,
) -> Result<Status, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    let admin_gamertag = admin.player.gamertag.as_deref().unwrap_or_default();
    let targeting_self = admin_gamertag == req.gamertag && admin.player.game == req.game;
    let targeting_admin_perm = req.permission == Permission::Admin.as_str();
    if targeting_self && targeting_admin_perm {
        tracing::warn!(
            "clear_permission: rejecting self-admin-clear by {} ({:?})",
            admin_gamertag,
            admin.player.game,
        );
        return Err(Status::Conflict);
    }

    let player_record = player::Entity::find()
        .filter(player::Column::Gamertag.eq(req.gamertag.clone()))
        .filter(player::Column::Game.eq(req.game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            tracing::error!("clear_permission: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    let removed = PermissionService::clear_override(conn, player_record.id, &req.permission)
        .await
        .map_err(|e| {
            tracing::error!("clear_permission: db error: {}", e);
            Status::InternalServerError
        })?;

    if removed {
        Ok(Status::NoContent)
    } else {
        Err(Status::NotFound)
    }
}
