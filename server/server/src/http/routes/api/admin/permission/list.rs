use common::Game;
use common::response::admin::{PermissionEntry, PermissionListResponse};
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::PermissionService;

#[openapi(tag = "Admin")]
#[get("/permission/<game>/<gamertag>")]
pub async fn list_permissions(
    _admin: AdminGuard,
    db: Db<'_>,
    game: Game,
    gamertag: &str,
) -> Result<Json<PermissionListResponse>, Status> {
    let conn = db.into_inner();

    let player_record = player::Entity::find()
        .filter(player::Column::Gamertag.eq(gamertag))
        .filter(player::Column::Game.eq(game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            tracing::error!("list_permissions: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    let entries = PermissionService::list_overrides(conn, player_record.id)
        .await
        .map_err(|e| {
            tracing::error!("list_permissions: db error: {}", e);
            Status::InternalServerError
        })?
        .into_iter()
        .map(|(permission, effect)| PermissionEntry { permission, effect })
        .collect();

    Ok(Json(PermissionListResponse {
        gamertag: gamertag.to_string(),
        game,
        entries,
    }))
}
