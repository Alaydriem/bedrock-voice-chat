use common::request::admin::SetPermissionRequest;
use common::structs::permission::{Permission, PermissionEffect};
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::{PermissionService, PermissionServiceError};

/// Record an explicit allow or deny permission override for a player.
///
/// Overrides take precedence over the server-wide defaults in the `permissions`
/// config block. Use DELETE to remove one.
#[openapi(tag = "Admin")]
#[put("/permission", data = "<payload>")]
pub async fn set_permission(
    admin: AdminGuard,
    db: Db<'_>,
    payload: Json<SetPermissionRequest>,
) -> Result<Status, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    crate::http::routes::api::admin::validate_gamertag(&req.gamertag)?;

    let admin_gamertag = admin.player.gamertag.as_deref().unwrap_or_default();
    let targeting_self = admin_gamertag == req.gamertag && admin.player.game == req.game;
    let targeting_admin_perm = req.permission == Permission::Admin.as_str();
    let revoking = matches!(req.effect, PermissionEffect::Deny);
    if targeting_self && targeting_admin_perm && revoking {
        tracing::warn!(
            "set_permission: rejecting self-admin-revoke by {} ({:?})",
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
