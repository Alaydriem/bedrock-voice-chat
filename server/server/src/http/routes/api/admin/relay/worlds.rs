use common::response::admin::{RelayWorld, RelayWorldsResponse};
use rocket::{State, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::stream::quic::CacheManager;

/// List the relay worlds this server currently has players in.
///
/// A relay world id is chosen by the game-side mod and never persisted here, so
/// this reflects live presence: a world with nobody in it is absent, not zero.
#[openapi(tag = "Admin")]
#[get("/relay/worlds")]
pub async fn relay_worlds(
    _admin: AdminGuard,
    cache_manager: &State<CacheManager>,
) -> Json<RelayWorldsResponse> {
    let worlds = cache_manager
        .relay_world_populations()
        .into_iter()
        .map(|(world, players)| RelayWorld { world, players })
        .collect();

    Json(RelayWorldsResponse { worlds })
}
