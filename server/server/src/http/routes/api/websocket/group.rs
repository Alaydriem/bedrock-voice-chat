use common::curia;
use std::sync::Arc;
use std::time::Instant;

use common::structs::group_key::GroupKeyFrame;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket::http::Status;
use rocket_ws::frame::{CloseCode, CloseFrame};
use rocket_ws::{Channel, Message, WebSocket};

use crate::http::guards::PlayerGuard;
use crate::services::group_key_relay_service::{CloseOutcome, RelayOutcome};
use crate::services::{ChannelMembershipService, GroupKeyRelayService};
use crate::stream::quic::{CacheManager, WebhookReceiver};

/// Close reason a joiner reads as "no member is online to admit you".
pub const NO_ADMITTER: &str = "NoAdmitterOnline";

/// A protected group's key socket.
///
/// Identity is the client certificate, as on every player route — not a ticket. Issuing a ticket
/// revokes the identity's previous unredeemed one, so this socket and the positions feed
/// reconnecting together would race. Returns a bare `Channel`: the app offers no subprotocol.
///
/// The server only sequences and routes. When the last socket closes, nobody holds the group's
/// keys any more, so the channel is deleted with it.
#[get("/websocket/group/<id>")]
pub async fn group_key_socket(
    ws: WebSocket,
    guard: PlayerGuard,
    id: &str,
    relay: &State<Arc<GroupKeyRelayService>>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
) -> Result<Channel<'static>, Status> {
    let gamertag = guard.player.gamertag.clone().ok_or(Status::Forbidden)?;
    let identity = guard.player.game.membership_key(&gamertag);
    let channels = cache_manager.get_channel_collection();

    let channel = channels.get(id).await.ok_or(Status::NotFound)?;
    if !channel.is_protected() {
        return Err(Status::Conflict);
    }
    if !channel.contains(&identity) {
        return Err(Status::Forbidden);
    }

    let relay = Arc::clone(relay);
    let webhook = webhook_receiver.inner().clone();
    let group = id.to_string();

    Ok(ws.channel(move |mut stream| {
        Box::pin(async move {
            let (socket, mut rx) = relay.open(&group, identity).await;
            let mut close_reason: Option<&'static str> = None;

            loop {
                tokio::select! {
                    inbound = stream.next() => {
                        let Some(Ok(message)) = inbound else {
                            break;
                        };
                        let Message::Binary(bytes) = message else {
                            continue;
                        };
                        let Ok(frame) = GroupKeyFrame::from_bytes(&bytes) else {
                            curia::debug!("group key socket sent an undecodable frame", { "group": group.clone() });
                            continue;
                        };
                        if relay.handle(&group, socket, frame, Instant::now()).await
                            == RelayOutcome::NoAdmitter
                        {
                            close_reason = Some(NO_ADMITTER);
                            break;
                        }
                    }

                    outbound = rx.recv() => {
                        // `None`: the relay dropped this socket (displaced, rate-limited, or too
                        // slow to drain its queue).
                        let Some(frame) = outbound else {
                            break;
                        };
                        let Ok(bytes) = frame.to_bytes() else {
                            break;
                        };
                        if stream.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                }
            }

            if let Some(reason) = close_reason {
                let _ = stream
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Policy,
                        reason: reason.into(),
                    })))
                    .await;
            }

            if relay.close(&group, socket).await == CloseOutcome::GroupEmpty {
                if let Some(channel) = channels.get(&group).await {
                    ChannelMembershipService::close(&channels, &webhook, &channel.creator, &group)
                        .await;
                    curia::info!("protected group ended with its last key socket", { "group": group.clone() });
                }
            }

            Ok(())
        })
    }))
}
