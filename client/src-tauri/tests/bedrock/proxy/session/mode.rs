use std::sync::Arc;

use bvc_client_lib::NetworkPacket;
use bvc_client_lib::bedrock::proxy::session::{
    BedrockSessionEventDispatcher, BedrockSessionState, DispatchOutcome,
};
use bvc_client_lib::bedrock::{
    BedrockChatChannel, BedrockEventEmitter, BedrockPlayerStateCache, JukeboxBeaconCache,
};
use common::bedrock_protocol::version::ProtocolVersion;
use common::structs::bedrock::AddonMode;

use crate::bedrock::proxy::session::fixture::EventFixture;

fn dispatcher(mode: AddonMode, chat: Arc<BedrockChatChannel>) -> BedrockSessionEventDispatcher {
    let (tx, _rx) = flume::unbounded::<NetworkPacket>();
    BedrockSessionEventDispatcher::new(
        "Alice".to_string(),
        Arc::new(JukeboxBeaconCache::new()),
        Arc::new(BedrockPlayerStateCache::new()),
        Some(Arc::new(BedrockEventEmitter::new(Arc::new(tx)))),
        bvc_client_lib::control::ControlActionSender::channel().0,
        bvc_client_lib::control::ControlStateBus::new(),
        Some(chat),
        mode,
    )
}

// Both modes must end the session on a peer disconnect. Without this the loop
// never breaks and the disconnect telemetry has no reason.
#[test]
fn both_modes_end_the_session_on_disconnect() {
    for mode in [AddonMode::Net, AddonMode::NoNet] {
        let chat = Arc::new(BedrockChatChannel::new());
        let mut d = dispatcher(mode, Arc::clone(&chat));
        let mut state = BedrockSessionState::new("Alice".to_string(), None);
        let outcome = d.dispatch(&EventFixture::disconnect(ProtocolVersion::LATEST), &mut state);
        assert!(
            matches!(outcome, DispatchOutcome::SessionEnded { .. }),
            "{mode:?} must end the session on disconnect"
        );
    }
}

// StartGame is connection setup, so relay-only still derives the world id.
#[test]
fn relay_only_still_derives_the_world_id() {
    let chat = Arc::new(BedrockChatChannel::new());
    let mut d = dispatcher(AddonMode::Net, chat);
    let mut state = BedrockSessionState::new("Alice".to_string(), None);
    d.dispatch(&EventFixture::start_game(ProtocolVersion::LATEST), &mut state);
    assert!(
        state.world_uuid().is_some(),
        "relay-only must still apply StartGame"
    );
}

// Chat ingress is the addon's job on a net world. Two deliveries of every
// message is the split brain this mode exists to close.
#[test]
fn relay_only_does_not_forward_chat() {
    let chat = Arc::new(BedrockChatChannel::new());
    let mut rx = chat.sender().subscribe();
    let mut d = dispatcher(AddonMode::Net, Arc::clone(&chat));
    let mut state = BedrockSessionState::new("Alice".to_string(), None);
    d.dispatch(
        &EventFixture::chat(ProtocolVersion::LATEST, "hello world"),
        &mut state,
    );
    assert!(
        rx.try_recv().is_err(),
        "relay-only must not forward chat to the app"
    );
}

#[test]
fn full_forwards_chat() {
    let chat = Arc::new(BedrockChatChannel::new());
    let mut rx = chat.sender().subscribe();
    let mut d = dispatcher(AddonMode::NoNet, Arc::clone(&chat));
    let mut state = BedrockSessionState::new("Alice".to_string(), None);
    d.dispatch(
        &EventFixture::chat(ProtocolVersion::LATEST, "hello world"),
        &mut state,
    );
    assert!(
        rx.try_recv().is_ok(),
        "full processing must forward chat to the app"
    );
}
