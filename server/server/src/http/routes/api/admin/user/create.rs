use std::sync::Arc;

use common::request::admin::CreateUserRequest;
use common::response::admin::CreatedUserResponse;
use entity::player;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::{CertificateService, PlayerRegistrarService};

#[openapi(tag = "Admin")]
#[post("/user", data = "<payload>")]
pub async fn create_user(
    _admin: AdminGuard,
    db: Db<'_>,
    payload: Json<CreateUserRequest>,
    cert_service: &State<Arc<CertificateService>>,
) -> Result<(Status, Json<CreatedUserResponse>), Status> {
    let conn = db.into_inner();
    let req = payload.0;

    crate::http::routes::api::admin::validate_gamertag(&req.gamertag)?;

    let existing = player::Entity::find()
        .filter(player::Column::Gamertag.eq(req.gamertag.clone()))
        .filter(player::Column::Game.eq(req.game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            tracing::error!("create_user: db error checking existence: {}", e);
            Status::InternalServerError
        })?;

    if existing.is_some() {
        return Err(Status::Conflict);
    }

    let registrar =
        PlayerRegistrarService::new(Arc::new(conn.clone()), cert_service.inner().clone());
    registrar
        .create_player(&req.gamertag, &req.game, None)
        .await
        .map_err(|e| {
            tracing::error!("create_user: failed to create player: {}", e);
            Status::InternalServerError
        })?;

    Ok((
        Status::Created,
        Json(CreatedUserResponse {
            gamertag: req.gamertag,
            game: req.game,
        }),
    ))
}
