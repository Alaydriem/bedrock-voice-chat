use common::curia;
use std::sync::Arc;

use common::structs::chat::ChatFrame;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{Channel, Message, WebSocket};

use crate::http::guards::GameAccessToken;
use crate::services::ChatService;

// Bounded. Chat that cannot keep up is dropped rather than buffered: a backlog delivered late
// lands stale lines in a conversation that has already moved on, and nothing here is stored.
const OUTBOUND_CAPACITY: usize = 64;

/// The mod's chat channel — one socket per world, both directions.
///
/// A mod can set request headers where a browser cannot, so identity is the access token on
/// the upgrade and no ticket is involved. The first frame must be `hello`; the world it names
/// is a property of the connection, so no later frame carries one.
///
/// Returns a bare `Channel`, deliberately — **not** the `ProtocolChannel` the positions feed
/// uses. That wrapper always writes `Sec-WebSocket-Protocol`, which is right there because a
/// browser sends its ticket as a subprotocol and demands the echo. These clients offer no
/// subprotocol at all, and RFC 6455 requires a client to fail the connection when the server
/// names one it did not offer. WinHTTP does exactly that, and BDS reports it as
/// `InternalWebSocketError 0x80072f78` — an unparseable handshake rather than a rejected one.
#[get("/websocket/chat")]
pub fn chat(
    ws: WebSocket,
    _access_token: GameAccessToken,
    chat_service: &State<Arc<ChatService>>,
) -> Channel<'static> {
    let service = Arc::clone(chat_service);

    ws.channel(move |mut stream| {
        Box::pin(async move {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(OUTBOUND_CAPACITY);
            // Handed to the registry on `hello` and not retained here, so that the registry
            // holds the only sender. Displacing this socket then closes `rx`, which is what
            // ends the loop below. Keeping a copy would leave the displaced socket running
            // forever: unregistered, invisible, and still connected.
            let mut tx = Some(tx);
            // Every id this socket is registered under. Usually one; several when a mod
            // covers a dimension-per-id server.
            let mut registered: Vec<String> = Vec::new();
            let socket_id = service.next_socket_id();

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
                            curia::debug!("chat socket sent an undecodable frame");
                            continue;
                        };

                        match frame {
                            ChatFrame::Hello { world: uuid, world_name, worlds, .. } => {
                                // One hello per socket, enforced by the sender being taken:
                                // a second is a confused mod, and honouring it would silently
                                // move the registration.
                                let Some(sender) = tx.take() else {
                                    continue;
                                };

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
                                // all of them. Dropping the displaced senders closes those
                                // sockets, because the registry holds the only one each has:
                                // two registrations for one id doubles every message.
                                for previous in
                                    service.register_room(socket_id, &keys, world_name, sender)
                                {
                                    drop(previous);
                                }
                                registered = keys;

                                // Without this an operator sees a healthy socket carrying
                                // nothing, which is indistinguishable from a fault.
                                if !service.is_enabled() {
                                    curia::info!("chat is disabled; frames on this socket will be dropped", { "worlds": format!("{registered:?}"), "socket": socket_id });
                                }
                            }
                            ChatFrame::Chat { author, text } => {
                                if registered.is_empty() {
                                    // Anything before `hello` has no world to belong to.
                                    break;
                                }
                                service.on_game_chat(&registered, author, text).await;
                            }
                            ChatFrame::Event { text } => {
                                if registered.is_empty() {
                                    break;
                                }
                                service.on_game_event(&registered, text).await;
                            }
                            // Server to mod only. A mod sending one is confused, not hostile.
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
                service.unregister(uuid, socket_id);
            }
            Ok(())
        })
    })
}
