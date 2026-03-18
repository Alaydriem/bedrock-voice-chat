use crate::rs::pool::AppDb;
use crate::services::GamerpicDecoder;
use common::Game;
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
    game: Game,
    gamertag: &str,
) -> status::Custom<Json<GamerpicResponse>> {
    let conn = db.into_inner();

    let result = player::Entity::find()
        .filter(player::Column::Gamertag.eq(gamertag))
        .filter(player::Column::Game.eq(game.clone()))
        .one(conn)
        .await;

    let gamerpic = match result {
        Ok(Some(record)) => GamerpicDecoder::decode(record.gamerpic),
        _ => None,
    };

    status::Custom(
        Status::Ok,
        Json(GamerpicResponse {
            gamertag: gamertag.to_string(),
            game,
            gamerpic,
        }),
    )
}
