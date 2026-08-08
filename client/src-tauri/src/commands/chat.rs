use common::structs::chat::ChatAvailability;
#[cfg(feature = "bedrock-protocol")]
use common::traits::StreamTrait;
use tauri::State;
use tokio::sync::Mutex;

#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::BedrockState;

/// Whether chat can carry a message right now.
///
/// One answer covering every source, not a flag per transport: a proxy session and a mod
/// reporting positions are two routes to the same world, and the composer only needs to know
/// whether typing will accomplish anything.
///
/// Polled rather than pushed. Liveness has several independent inputs — a proxy starting, a
/// session ending, positions arriving — and a single missed event on any of them would leave
/// the composer permanently wrong in one direction. A poll self-corrects; an event stream that
/// drops one does not.
#[cfg(feature = "bedrock-protocol")]
#[tauri::command]
pub(crate) async fn chat_availability(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<ChatAvailability, String> {
    let state = state.lock().await;

    let proxy_live = state.proxy.as_ref().is_some_and(|p| !p.is_stopped());
    let realm_live = state.realms.as_ref().is_some_and(|r| !r.is_stopped());

    if proxy_live || realm_live {
        // The host the user chose is the most meaningful name available on this path: the
        // world's own name is a per-session Bedrock field the manager does not surface, and
        // `world_uuid` is never shown.
        return Ok(ChatAvailability::local(state.proxy_target_host.clone()));
    }

    // Phase 2 adds the server-hub branch here: a world reporting positions with a registered
    // chat channel is reachable without any proxy at all.
    Ok(ChatAvailability::unavailable("Not connected to a world"))
}

#[cfg(not(feature = "bedrock-protocol"))]
#[tauri::command]
pub(crate) async fn chat_availability() -> Result<ChatAvailability, String> {
    Ok(ChatAvailability::unavailable("Chat is not available"))
}

/// Sends a line from the app into a net-mode world.
///
/// The world is named by the caller and validated server-side against where the player is
/// actually standing: they may have been transferred while the app still held the older
/// target, and delivering there would put the message in front of people they are not with.
#[tauri::command]
pub(crate) async fn chat_send(
    world_uuid: Option<String>,
    text: String,
    producer: State<'_, std::sync::Arc<flume::Sender<crate::NetworkPacket>>>,
) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("Nothing to send.".to_string());
    }

    let packet = crate::NetworkPacket {
        data: common::structs::packet::QuicNetworkPacket {
            packet_type: common::structs::packet::PacketType::ChatSend,
            // The server stamps the sender from the connection's mTLS identity. Anything set
            // here would be ignored, and trusting it would let a client post as anyone.
            sender: None,
            data: common::structs::packet::QuicNetworkPacketData::ChatSend(
                common::structs::packet::ChatSendPacket::new(world_uuid, text),
            ),
            ..Default::default()
        },
    };

    producer
        .send(packet)
        .map_err(|e| format!("Could not reach the server: {e}"))
}

/// Worlds this player has been seen in, for the composer's target picker.
///
/// Net mode only. The no-net path has no world key — the proxy session is the world — so this
/// returns empty rather than erroring there.
#[tauri::command]
pub(crate) async fn chat_worlds(
    state: State<'_, Mutex<crate::structs::app_state::AppState>>,
) -> Result<Vec<common::structs::chat::ChatWorld>, String> {
    let state = state.lock().await;
    let Some(api) = state.api_client.as_ref() else {
        return Ok(Vec::new());
    };
    api.chat_worlds().await
}
