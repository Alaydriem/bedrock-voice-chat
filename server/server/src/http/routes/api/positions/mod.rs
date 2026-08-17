use std::sync::Arc;

use common::Game;
use common::traits::player_data::PlayerData;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::runtime::position_updater;
use crate::{
    http::guards::GameAccessToken,
    services::{BedrockEventService, PlayerIdentityService, PlayerRegistrarService},
    stream::quic::{CacheManager, WebhookReceiver},
};

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
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: update_position, position]
        },
    }
}

/// Report player positions from the game server.
///
/// Called by the Addon or Java mod on every position update, authenticated with the
/// shared access token. This is what makes proximity audio possible.
#[openapi(tag = "Positions")]
#[post("/position", data = "<positions>")]
pub async fn update_position(
    _access_token: GameAccessToken,
    positions: Json<common::GameDataCollection>,
    webhook_receiver: &State<WebhookReceiver>,
    player_registrar: &State<PlayerRegistrarService>,
    identity_service: &State<PlayerIdentityService>,
    bedrock_event_service: &State<Arc<BedrockEventService>>,
) -> Status {
    let game_type = positions.0.game.clone().unwrap_or(Game::Minecraft);

    let mut all_players: Vec<_> = positions.0.players.clone();

    let mut seen_worlds: std::collections::HashSet<String> = std::collections::HashSet::new();
    for player in &all_players {
        if let common::PlayerEnum::Minecraft(mc) = player {
            if let Some(world_uuid) = &mc.world_uuid {
                if !world_uuid.is_empty() && seen_worlds.insert(world_uuid.clone()) {
                    bedrock_event_service.notify_addon_http(world_uuid).await;
                }
            }
        }
    }

    for player in &all_players {
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

/// Return the positions the server currently holds.
///
/// Useful for confirming the game server is actually relaying: an empty response means
/// it is not.
#[openapi(tag = "Positions")]
#[get("/position")]
pub async fn position(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
) -> Json<Vec<common::PlayerEnum>> {
    let player_cache = cache_manager.players().inner_arc();

    let mut players = Vec::new();

    for (_, player) in player_cache.iter() {
        players.push(player.clone());
    }

    Json(players)
}
