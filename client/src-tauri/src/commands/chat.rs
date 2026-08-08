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
