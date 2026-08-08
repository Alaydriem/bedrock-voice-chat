use std::sync::Arc;

use common::structs::chat::ChatFrame;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{Message, WebSocket};

use super::protocol_channel::ProtocolChannel;
use crate::http::guards::MCAccessToken;
use crate::services::ChatService;

// Echoed back at the handshake so a client offering a subprotocol gets one named.
const PROTOCOL: &str = "bvc.chat.v1";

// Bounded. Chat that cannot keep up is dropped rather than buffered: a backlog delivered late
// lands stale lines in a conversation that has already moved on, and nothing here is stored.
const OUTBOUND_CAPACITY: usize = 64;

/// The mod's chat channel — one socket per world, both directions.
///
/// A mod can set request headers where a browser cannot, so identity is the access token on
/// the upgrade and no ticket is involved. The first frame must be `hello`; the world it names
/// is a property of the connection, so no later frame carries one.
#[get("/websocket/chat")]
pub fn chat(
    ws: WebSocket,
    _access_token: MCAccessToken,
    chat_service: &State<Arc<ChatService>>,
) -> ProtocolChannel<'static> {
    let service = Arc::clone(chat_service);

    let channel = ws.channel(move |mut stream| {
        Box::pin(async move {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(OUTBOUND_CAPACITY);
            // Every id this socket is registered under. Usually one; several when a mod
            // covers a dimension-per-id server.
            let mut registered: Vec<String> = Vec::new();

            loop {
                tokio::select! {
                    inbound = stream.next() => {
                        let Some(Ok(message)) = inbound else {
                            break;
                        };
                        let Message::Text(body) = message else {
                            // Binary and control frames are not part of this protocol.
                            continue;
                        };
                        let Ok(frame) = serde_json::from_str::<ChatFrame>(&body) else {
                            tracing::debug!("chat socket sent an undecodable frame");
                            continue;
                        };

                        match frame {
                            ChatFrame::Hello { world: uuid, world_name, worlds, .. } => {
                                if !registered.is_empty() {
                                    // One hello per socket. A second is a confused mod, and
                                    // honouring it would silently move the registration.
                                    continue;
                                }

                                // The canonical id first, then any extra ids the same room
                                // spans. Deduplicated so a mod listing its primary twice does
                                // not register it twice.
                                let mut keys = vec![uuid.clone()];
                                for extra in worlds {
                                    if !keys.contains(&extra) {
                                        keys.push(extra);
                                    }
                                }

                                // Registered as one room so a lookup by any id answers for
                                // all of them. Displaced sockets are dropped rather than left
                                // running: two registrations for one id doubles every message.
                                for previous in
                                    service.register_room(&keys, world_name, tx.clone())
                                {
                                    drop(previous);
                                }
                                registered = keys;
                            }
                            ChatFrame::Chat { author, text } => {
                                if registered.is_empty() {
                                    // Anything before `hello` has no world to belong to.
                                    break;
                                }
                                service.on_game_chat(&registered, author, text).await;
                            }
                            // Server-authored. A mod sending one is confused, not hostile.
                            ChatFrame::Say { .. } => continue,
                        }
                    }

                    outbound = rx.recv() => {
                        let Some(body) = outbound else {
                            break;
                        };
                        if stream.send(Message::Text(body)).await.is_err() {
                            break;
                        }
                    }
                }
            }

            for uuid in &registered {
                service.unregister(uuid);
            }
            Ok(())
        })
    });

    ProtocolChannel::new(channel, PROTOCOL)
}
