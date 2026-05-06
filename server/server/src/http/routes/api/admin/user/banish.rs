use common::request::admin::BanishUserRequest;
use common::response::admin::BanishedUserResponse;
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;

#[openapi(tag = "Admin")]
#[patch("/user/banish", data = "<payload>")]
pub async fn banish_user(
    admin: AdminGuard,
    db: Db<'_>,
    payload: Json<BanishUserRequest>,
) -> Result<Json<BanishedUserResponse>, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    crate::http::routes::api::admin::validate_gamertag(&req.gamertag)?;

    let admin_gamertag = admin.player.gamertag.as_deref().unwrap_or_default();
    if admin_gamertag == req.gamertag && admin.player.game == req.game {
        tracing::warn!(
            "banish_user: rejecting self-banish attempt by {} ({:?})",
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
            tracing::error!("banish_user: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    let mut active: player::ActiveModel = player_record.into();
    active.banished = ActiveValue::Set(req.banish);

    active.update(conn).await.map_err(|e| {
        tracing::error!("banish_user: update failed: {}", e);
        Status::InternalServerError
    })?;

    Ok(Json(BanishedUserResponse {
        gamertag: req.gamertag,
        game: req.game,
        banished: req.banish,
    }))
}
