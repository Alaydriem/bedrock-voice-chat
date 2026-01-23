use common::Game;
use rocket::{http::Status, serde::json::Json, State};

use crate::{
    rs::guards::MCAccessToken,
    services::PlayerRegistrarService,
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
) -> Status {
    // Get the game type from the request, defaulting to Minecraft for backwards compatibility
    let game_type = positions.0.game.clone().unwrap_or(Game::Minecraft);

    // Collect all players for broadcasting
    let all_players: Vec<_> = positions.0.players.clone();

    // Process players through the registrar (checks cache, creates new players as needed)
    player_registrar.process_players(&all_players, game_type).await;

    // Broadcast positions to connected QUIC clients
    position_updater::broadcast_positions(all_players, webhook_receiver).await;

    Status::Ok
}

#[get("/position")]
pub async fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Cache manager state for accessing player positions
    cache_manager: &State<CacheManager>,
) -> Json<Vec<common::PlayerEnum>> {
    // Get all current player positions from the cache
    let player_cache = cache_manager.get_player_cache();

    // Collect all cached players
    let mut players = Vec::new();

    for (_, player) in player_cache.iter() {
        players.push(player.clone());
    }

    // Return the collected players
    Json(players)
}
