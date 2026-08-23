use common::curia;
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

// Echoed back at the handshake. The client offers this alongside its ticket so
// the server has a subprotocol to accept that is not the credential itself.
const PROTOCOL: &str = "bvc.positions.v1";

// Sockets are recycled rather than living forever, so a stale identity cannot
// hold an open feed indefinitely.
const SESSION_MAX: Duration = Duration::from_secs(6 * 60 * 60);

/// Streams the caller's view of the players around them.
///
/// Identity comes from the redeemed ticket and nothing else: the client cannot
/// name a player, world or dimension, so it cannot request anyone else's view.
///
/// The socket is held for as long as the client holds it. An observer the world does not
/// contain is a normal, recoverable state -- signed in ahead of joining, mid respawn, between
/// worlds -- and closing on it was self-defeating: each ticket is single-use and issuing one
/// revokes the identity's previous, so a client reconnecting on a timer spends its life
/// swapping credentials, and any overlap between the closing socket and the opening one leaves
/// the newcomer holding a ticket that has already been superseded. Empty frames cost a few
/// bytes twice a second and keep the roster live the instant somebody walks up.
#[get("/websocket/positions")]
pub fn positions(
    ws: WebSocket,
    ticket: WebsocketTicket,
    feed: &State<Arc<PositionFeedService>>,
    voice: &State<Voice>,
) -> ProtocolChannel<'static> {
    let feed = Arc::clone(feed);
    let voice_range = voice.spatial_audio.broadcast_range;
    // Redeemed by the guard, so an upgrade that reaches here is already authenticated.
    let WebsocketTicket(identity) = ticket;
    // Composed once, outside the loop. The world index keys on the canonical identity, and the
    // ticket carries the game and the gamertag apart.
    let observer_identity = identity
        .game
        .membership_key(&identity.gamertag)
        .to_string();

    let channel = ws.channel(move |mut stream| {
        Box::pin(async move {
            let service = PositionService::for_voice_range(voice_range);
            // Driven by the index rather than by a clock of its own, so a snapshot leaves as
            // soon as the picture it describes exists.
            let mut index_rx = feed.subscribe();
            let started = Instant::now();
            let mut seq = 0u64;

            loop {
                // Only fails once every sender is gone, which cannot happen while the service
                // is managed — and if it did there would be nothing left to report.
                if index_rx.changed().await.is_err() {
                    break;
                }

                if started.elapsed() >= SESSION_MAX {
                    curia::debug!("position session reached its maximum lifetime", { "gamertag": identity.gamertag.to_string() });
                    break;
                }

                seq += 1;

                // The index is built once per tick for every socket. Reading it here is a
                // lookup plus the trigonometry for this observer's own neighbours, which is
                // the part that genuinely cannot be shared.
                let index = index_rx.borrow_and_update().clone();
                let snapshot = match index.observer(&observer_identity) {
                    Some(observer) => {
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
