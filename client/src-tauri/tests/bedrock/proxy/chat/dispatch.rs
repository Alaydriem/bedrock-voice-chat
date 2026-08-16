use std::sync::Arc;

use bvc_client_lib::bedrock::proxy::session::{BedrockSessionEventDispatcher, BedrockSessionState};
use bvc_client_lib::bedrock::{BedrockChatChannel, BedrockPlayerStateCache};
use bvc_client_lib::bedrock::{BedrockEventEmitter, JukeboxBeaconCache};
use common::bedrock_protocol::protocol::event::EventPacket;
use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::{
    AuthorAndMessage, TextPacketBody, TextPacketType,
};
use common::bedrock_protocol::{Direction, Event, ProtocolVersion};
use common::structs::bedrock::AddonMode;

/// Wires a dispatcher to a live chat channel and hands back a subscriber.
///
/// This is the seam that shipped broken once: the codec was correct and tested, the channel
/// existed, and nothing connected the two on the path a real session takes.
fn build() -> (
    BedrockSessionEventDispatcher,
    tokio::sync::broadcast::Receiver<bvc_client_lib::bedrock::ChatLine>,
) {
    let (tx, _rx) = flume::unbounded();
    let channel = Arc::new(BedrockChatChannel::new());
    let subscriber = channel.sender().subscribe();

    let dispatcher = BedrockSessionEventDispatcher::new(
        "Alaydriem".to_string(),
        Arc::new(JukeboxBeaconCache::new()),
        Arc::new(BedrockPlayerStateCache::new()),
        Some(Arc::new(BedrockEventEmitter::new(Arc::new(tx)))),
        bvc_client_lib::control::ControlActionSender::channel().0,
        bvc_client_lib::control::ControlStateBus::new(),
        Some(channel),
        // Chat ingress is the full-processing child's job; relay-only would
        // never reach the seam this test exists to cover.
        AddonMode::NoNet,
    );

    (dispatcher, subscriber)
}

fn chat_event(message: &str, direction: Direction) -> Event {
    let packet = TextPacket {
        localize: false,
        body: TextPacketBody::AuthorAndMessage(AuthorAndMessage {
            message_type: TextPacketType::Chat,
            player_name: "Petra".to_string(),
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

#[test]
fn an_ordinary_clientbound_chat_line_reaches_the_chat_channel() {
    let (mut dispatcher, mut rx) = build();
    let mut state = BedrockSessionState::new("Alaydriem".to_string(), None);

    dispatcher.dispatch(
        &chat_event("anyone got spare iron", Direction::Clientbound),
        &mut state,
    );

    let line = rx.try_recv().expect("the line should reach the channel");
    assert_eq!(line.author.as_deref(), Some("Petra"));
    assert_eq!(line.text, "anyone got spare iron");
}

// The proxy injects its own serverbound chat. Observing that direction as well would render
// every app-sent message twice: once on send, once on the realm's echo.
#[test]
fn serverbound_chat_is_not_relayed() {
    let (mut dispatcher, mut rx) = build();
    let mut state = BedrockSessionState::new("Alaydriem".to_string(), None);

    dispatcher.dispatch(&chat_event("hello", Direction::Serverbound), &mut state);

    assert!(rx.try_recv().is_err());
}

