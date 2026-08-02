use common::response::websocket::WebsocketTicketResponse;
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::pool::Db;
use crate::services::AuthService;
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
    cert: Certificate<'_>,
    db: Db<'_>,
    cache_manager: &State<CacheManager>,
) -> Result<Json<WebsocketTicketResponse>, Status> {
    let conn = db.into_inner();
    let player = AuthService::player_from_certificate(&cert, conn, None).await?;

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
