#![allow(dead_code)]

use bedrock_server::{StartGameConfig, TextPacketConfig};
use common::bedrock_protocol::protocol::event::{DisconnectReason, EventPacket};
use common::bedrock_protocol::version::ProtocolVersion;
use common::bedrock_protocol::{Direction, Event};

// Builders for the three events the mode split branches on. `Event::new` is
// public, so a test can construct one without a live session; the payloads reuse
// the same `bedrock_server` config helpers the fake upstream drives real
// scenarios with, so a fixture cannot drift from what the proxy actually sees.
//
// Clientbound throughout: that is the only direction the dispatcher acts on for
// chat, and StartGame and Disconnect only ever arrive that way.
pub struct EventFixture;

impl EventFixture {
    pub fn start_game(version: ProtocolVersion) -> Event {
        Event::new(
            version,
            Direction::Clientbound,
            EventPacket::StartGame(StartGameConfig::for_version(version).into_packet()),
        )
    }

    pub fn disconnect(version: ProtocolVersion) -> Event {
        Event::new(
            version,
            Direction::Clientbound,
            EventPacket::Disconnected(DisconnectReason::ClientDisconnected),
        )
    }

    pub fn chat(version: ProtocolVersion, message: &str) -> Event {
        Event::new(
            version,
            Direction::Clientbound,
            EventPacket::ChatMessage(TextPacketConfig::chat(message).into_packet()),
        )
    }
}
