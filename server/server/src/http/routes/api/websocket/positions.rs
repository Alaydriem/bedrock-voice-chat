use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::position::PositionSnapshot;
use rocket::State;
use rocket::futures::SinkExt;
use rocket_ws::{Message, WebSocket};

use super::protocol_channel::ProtocolChannel;
use crate::config::Voice;
use crate::http::guards::WebsocketTicket;
use crate::services::{PositionFeedService, PositionService};
use crate::stream::quic::CacheManager;

// Echoed back at the handshake. The client offers this alongside its ticket so
// the server has a subprotocol to accept that is not the credential itself.
const PROTOCOL: &str = "bvc.positions.v1";

// Sockets are recycled rather than living forever, so a stale identity cannot
// hold an open feed indefinitely.
const SESSION_MAX: Duration = Duration::from_secs(6 * 60 * 60);

// Thirty seconds of an observer the world does not contain. Somebody who has left produces
// empty frames forever, and a socket that keeps sending them is a session nobody is on the
// other end of. Closing is also how the client learns to stop: it reconnects with a fresh
// ticket when it next has something to show.
//
// Counted only for an absent observer, never for a present one who happens to be alone.
// Proximity voice means being alone constantly — mining, exploring, anywhere off the beaten
// path — and closing those feeds put every one of those players behind a reconnect at the
// moment somebody finally walked up to them.
const ABSENT_SNAPSHOT_LIMIT: u32 = 60;

/// Streams the caller's view of the players around them.
///
/// Identity comes from the redeemed ticket and nothing else: the client cannot
/// name a player, world or dimension, so it cannot request anyone else's view.
#[get("/websocket/positions")]
pub fn positions(
    ws: WebSocket,
    ticket: WebsocketTicket,
    cache_manager: &State<CacheManager>,
    feed: &State<Arc<PositionFeedService>>,
    voice: &State<Voice>,
) -> ProtocolChannel<'static> {
    let cache_manager = (*cache_manager).clone();
    let feed = Arc::clone(feed);
    let voice_range = voice.spatial_audio.broadcast_range;
    let WebsocketTicket(ticket) = ticket;

    let channel = ws.channel(move |mut stream| {
        Box::pin(async move {
            let Some(identity) = cache_manager.websocket_tickets().redeem(&ticket).await else {
                tracing::debug!("position socket presented an unknown or spent ticket");
                return Ok(());
            };

            let service = PositionService::for_voice_range(voice_range);
            // Driven by the index rather than by a clock of its own, so a snapshot leaves as
            // soon as the picture it describes exists.
            let mut index_rx = feed.subscribe();
            let started = Instant::now();
            let mut seq = 0u64;
            let mut absent_runs = 0u32;

            loop {
                // Only fails once every sender is gone, which cannot happen while the service
                // is managed — and if it did there would be nothing left to report.
                if index_rx.changed().await.is_err() {
                    break;
                }

                if started.elapsed() >= SESSION_MAX {
                    tracing::debug!(
                        gamertag = %identity.gamertag,
                        "position session reached its maximum lifetime"
                    );
                    break;
                }

                seq += 1;

                // The index is built once per tick for every socket. Reading it here is a
                // lookup plus the trigonometry for this observer's own neighbours, which is
                // the part that genuinely cannot be shared.
                let index = index_rx.borrow_and_update().clone();
                let snapshot = match index.observer(&identity.gamertag) {
                    Some(observer) => {
                        absent_runs = 0;
                        let neighbours = index.neighbours(observer);
                        PositionSnapshot {
                            seq,
                            positions: service.snapshot_positions(observer, &neighbours, &|name| {
                                index.is_on_voice(name)
                            }),
                        }
                    }
                    // Authenticated but not in the game yet is a normal state, not an
                    // error: an empty frame keeps the UI live instead of looking broken.
                    None => {
                        absent_runs += 1;
                        PositionSnapshot {
                            seq,
                            positions: Vec::new(),
                        }
                    }
                };

                if absent_runs >= ABSENT_SNAPSHOT_LIMIT {
                    tracing::debug!(
                        gamertag = %identity.gamertag,
                        "position session's observer is not in the world; closing"
                    );
                    break;
                }

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
