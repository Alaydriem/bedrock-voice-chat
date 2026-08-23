use std::sync::Arc;

use bvc_client_lib::NetworkPacket;
use bvc_client_lib::bedrock::proxy::session::{BedrockSessionEventDispatcher, BedrockSessionState};
use bvc_client_lib::bedrock::{BedrockEventEmitter, JukeboxBeaconCache};
use bvc_client_lib::bedrock::BedrockPlayerStateCache;
use bvc_client_lib::control::{ControlActionSender, ControlStateBus};
use common::bedrock_protocol::Event;
use common::structs::bedrock::AddonMode;
use common::bedrock_protocol::Direction;
use common::bedrock_protocol::ProtocolVersion;
use common::bedrock_protocol::protocol::event::EventPacket;
use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::{
    AuthorAndMessage, TextPacketBody, TextPacketType,
};

fn chat_event(message: &str, direction: Direction) -> Event {
    let packet = TextPacket {
        localize: false,
        body: TextPacketBody::AuthorAndMessage(AuthorAndMessage {
            message_type: TextPacketType::Chat,
            player_name: "bob".to_string(),
            message: message.to_string(),
        }),
        sender_s_xuid: "xuid".to_string(),
        platform_id: String::new(),
        filtered_message: None,
    };
    Event::new(
        ProtocolVersion::LATEST,
        direction,
        EventPacket::ChatMessage(packet),
    )
}

// These exercise the observation and chat-ingress arms, which only the
// full-processing child owns.
fn build_dispatcher() -> (
    BedrockSessionEventDispatcher,
    flume::Receiver<NetworkPacket>,
) {
    let (tx, rx) = flume::unbounded::<NetworkPacket>();
    let emitter = Arc::new(BedrockEventEmitter::new(Arc::new(tx)));
    let dispatcher = BedrockSessionEventDispatcher::new(
        "alice".to_string(),
        Arc::new(JukeboxBeaconCache::new()),
        Arc::new(BedrockPlayerStateCache::new()),
        Some(emitter),
        ControlActionSender::channel().0,
        ControlStateBus::new(),
        None,
        AddonMode::NoNet,
    );
    (dispatcher, rx)
}

#[test]
fn clientbound_chat_emits_no_quic_packet() {
    let (mut dispatcher, rx) = build_dispatcher();
    let mut state = BedrockSessionState::new("alice".to_string(), None);

    let evt = chat_event("hello world", Direction::Clientbound);
    dispatcher.dispatch(&evt, &mut state);

    assert!(rx.try_recv().is_err());
}
