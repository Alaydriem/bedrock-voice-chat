use common::traits::player_data::PlayerData;
use common::Game;
use rocket::{http::Status, serde::json::Json, State};
use rocket_okapi::openapi;

use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::{
    http::guards::MCAccessToken,
    services::{PlayerIdentityService, PlayerRegistrarService},
    stream::quic::{CacheManager, WebhookReceiver},
};
use crate::runtime::position_updater;

inventory::submit! {
    TagDefinition {
        name: "Positions",
        description: "Player position updates from game mods. Used by the Minecraft/Hytale \
                      server plugin to push player coordinates for spatial audio calculations.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: update_position, position]
        },
    }
}

#[openapi(tag = "Positions")]
#[post("/position", data = "<positions>")]
pub async fn update_position(
    _access_token: MCAccessToken,
    positions: Json<common::GameDataCollection>,
    webhook_receiver: &State<WebhookReceiver>,
    player_registrar: &State<PlayerRegistrarService>,
    identity_service: &State<PlayerIdentityService>,
) -> Status {
    let game_type = positions.0.game.clone().unwrap_or(Game::Minecraft);

    let mut all_players: Vec<_> = positions.0.players.clone();

    for player in &all_players {
        let name = player.get_name();
        let alt = player.get_alternative_identity();

        if let Some(alt_identity) = alt {
            let name = player.get_name();
            if let Some(player_id) = identity_service
                .find_player_id_by_gamertag(alt_identity, &game_type)
                .await
            {
                let _ = identity_service
                    .create_alias(player_id, name, &game_type, "floodgate")
                    .await;
            }
        }
    }

    identity_service
        .resolve_and_remap_players(&mut all_players, &game_type)
        .await;

    player_registrar
        .process_players(&all_players, game_type)
        .await;

    position_updater::PositionUpdater::broadcast_positions(all_players, webhook_receiver).await;

    Status::Ok
}

#[openapi(tag = "Positions")]
#[get("/position")]
pub async fn position(
    _access_token: MCAccessToken,
    cache_manager: &State<CacheManager>,
) -> Json<Vec<common::PlayerEnum>> {
    let player_cache = cache_manager.get_player_cache();

    let mut players = Vec::new();

    for (_, player) in player_cache.iter() {
        players.push(player.clone());
    }

    Json(players)
}
