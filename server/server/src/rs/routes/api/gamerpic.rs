use crate::rs::pool::AppDb;
use common::response::GamerpicResponse;
use entity::player;
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json};
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use sea_orm_rocket::Connection as SeaOrmConnection;

#[get("/<game>/<gamertag>")]
pub async fn get_gamerpic<'r>(
    _identity: Certificate<'r>,
    db: SeaOrmConnection<'_, AppDb>,
    game: &str,
    gamertag: &str,
) -> status::Custom<Json<GamerpicResponse>> {
    let conn = db.into_inner();

    let parsed_game: common::Game = match game {
        "hytale" => common::Game::Hytale,
        _ => common::Game::Minecraft,
    };

    let result = player::Entity::find()
        .filter(player::Column::Gamertag.eq(gamertag))
        .filter(player::Column::Game.eq(parsed_game))
        .one(conn)
        .await;

    let gamerpic = match result {
        Ok(Some(record)) => record.gamerpic,
        _ => None,
    };

    status::Custom(
        Status::Ok,
        Json(GamerpicResponse {
            gamertag: gamertag.to_string(),
            game: game.to_string(),
            gamerpic,
        }),
    )
}
