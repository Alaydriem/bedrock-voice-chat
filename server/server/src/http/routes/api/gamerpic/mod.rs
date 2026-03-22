use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::http::pool::Db;
use crate::services::GamerpicDecoder;

inventory::submit! {
    TagDefinition {
        name: "Gamerpic",
        description: "Player avatar/gamerpic retrieval by gamertag and game type.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api/gamerpic",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: get_gamerpic]
        },
    }
}
use common::Game;
use common::response::GamerpicResponse;
use entity::player;
use rocket::mtls::Certificate;
use crate::http::openapi::CustomJsonResponseRequired;
use rocket_okapi::openapi;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;

#[openapi(tag = "Gamerpic")]
#[get("/<game>/<gamertag>")]
pub async fn get_gamerpic(
    _identity: Certificate<'_>,
    db: Db<'_>,
    game: Game,
    gamertag: &str,
) -> CustomJsonResponseRequired<GamerpicResponse> {
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

    CustomJsonResponseRequired::ok(GamerpicResponse {
        gamertag: gamertag.to_string(),
        game,
        gamerpic,
    })
}
