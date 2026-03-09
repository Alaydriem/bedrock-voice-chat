use common::traits::player_data::PlayerData;
use common::Game;
use rocket::{http::Status, serde::json::Json, State};

use crate::{
    rs::guards::MCAccessToken,
    services::{PlayerIdentityService, PlayerRegistrarService},
    stream::quic::{CacheManager, WebhookReceiver},
};

use crate::runtime::position_updater;

/// Stores player position data
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
        if let Some(alt_identity) = player.get_alternative_identity() {
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

    position_updater::broadcast_positions(all_players, webhook_receiver).await;

    Status::Ok
}

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
