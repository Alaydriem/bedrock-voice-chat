use std::time::{Duration, Instant};

use common::PlayerEnum;
use common::structs::position::PositionSnapshot;
use rocket::State;
use rocket::futures::SinkExt;
use rocket_ws::{Message, WebSocket};

use super::protocol_channel::ProtocolChannel;
use crate::config::Voice;
use crate::http::guards::WebsocketTicket;
use crate::services::{PositionHandle, PositionService};
use crate::stream::quic::CacheManager;

// Echoed back at the handshake. The client offers this alongside its ticket so
// the server has a subprotocol to accept that is not the credential itself.
const PROTOCOL: &str = "bvc.positions.v1";

// Sockets are recycled rather than living forever, so a stale identity cannot
// hold an open feed indefinitely.
const SESSION_MAX: Duration = Duration::from_secs(6 * 60 * 60);

// 2Hz. Independent of the 4Hz /api/position ingest: the feed samples the cache
// on its own timer rather than reacting to writes.
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500);

/// Streams the caller's anonymised view of the players around them.
///
/// Identity comes from the redeemed ticket and nothing else: the client cannot
/// name a player, world or dimension, so it cannot request anyone else's view.
#[get("/websocket/positions")]
pub fn positions(
    ws: WebSocket,
    ticket: WebsocketTicket,
    cache_manager: &State<CacheManager>,
    voice: &State<Voice>,
) -> ProtocolChannel<'static> {
    let cache_manager = (*cache_manager).clone();
    let voice_range = voice.spatial_audio.broadcast_range;
    let WebsocketTicket(ticket) = ticket;

    let channel = ws.channel(move |mut stream| {
        Box::pin(async move {
            let Some(identity) = cache_manager.websocket_tickets().redeem(&ticket).await else {
                tracing::debug!("position socket presented an unknown or spent ticket");
                return Ok(());
            };

            let service = PositionService::for_voice_range(voice_range);
            let handles = PositionHandle::new_session();
            let players = cache_manager.players().inner_arc();
            let mut ticker = tokio::time::interval(SNAPSHOT_INTERVAL);
            let started = Instant::now();
            let mut seq = 0u64;

            loop {
                ticker.tick().await;

                if started.elapsed() >= SESSION_MAX {
                    tracing::debug!(
                        gamertag = %identity.gamertag,
                        "position session reached its maximum lifetime"
                    );
                    break;
                }

                seq += 1;

                // Authenticated but not in the game yet is a normal state, not an
                // error: an empty frame keeps the UI live instead of looking broken.
                let snapshot = match players.get(&identity.gamertag).await {
                    Some(observer) => {
                        let world: Vec<PlayerEnum> =
                            players.iter().map(|(_, player)| player).collect();

                        PositionSnapshot {
                            seq,
                            positions: service.snapshot_positions(&observer, &world, &handles),
                        }
                    }
                    None => PositionSnapshot {
                        seq,
                        positions: Vec::new(),
                    },
                };

                let Ok(body) = serde_json::to_string(&snapshot) else {
                    break;
                };

                if stream.send(Message::Text(body)).await.is_err() {
                    break;
                }
            }

            Ok(())
        })
    });

    ProtocolChannel::new(channel, PROTOCOL)
}
