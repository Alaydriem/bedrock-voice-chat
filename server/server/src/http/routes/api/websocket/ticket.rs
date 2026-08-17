use crate::http::guards::PlayerGuard;
use common::response::websocket::WebsocketTicketResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::stream::quic::{CacheManager, TicketIdentity};

// Mirrors the cache's TTL, reported so the client need not hardcode it.
const TICKET_EXPIRES_IN: u64 = 60;

/// Exchanges the caller's mTLS identity for a single-use WebSocket ticket.
///
/// A browser cannot present a client certificate or set headers when opening a
/// socket, so identity is established here -- over the same mTLS every other
/// route uses -- and spent once at upgrade time.
#[openapi(tag = "WebSocket")]
#[post("/websocket/ticket")]
pub async fn ticket(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
) -> Result<Json<WebsocketTicketResponse>, Status> {
    let player = guard.player;

    let gamertag = player.gamertag.clone().ok_or(Status::Forbidden)?;

    let ticket = cache_manager
        .websocket_tickets()
        .issue(TicketIdentity {
            gamertag,
            game: player.game,
        })
        .await;

    Ok(Json(WebsocketTicketResponse {
        ticket,
        expires_in: TICKET_EXPIRES_IN,
    }))
}
