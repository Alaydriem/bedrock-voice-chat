use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::bedrock_event::BedrockEvent;
use super::bedrock_event_direction::BedrockEventDirection;
use super::quic_network_packet_data::QuicNetworkPacketData;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BedrockEventPacket {
    pub event: BedrockEvent,
    pub world_uuid: String,
    pub occurred_at_ms: u64,
    #[serde(default)]
    pub direction: BedrockEventDirection,
}

impl BedrockEventPacket {
    pub fn new(event: BedrockEvent, world_uuid: String) -> Self {
        Self::with_direction(event, world_uuid, BedrockEventDirection::ServerBound)
    }

    pub fn with_direction(
        event: BedrockEvent,
        world_uuid: String,
        direction: BedrockEventDirection,
    ) -> Self {
        Self {
            event,
            world_uuid,
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            direction,
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for BedrockEventPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::BedrockEvent(c) => Ok(c),
            _ => Err(()),
        }
    }
}
