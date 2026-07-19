use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::packet_direction::PacketDirection;
use super::quic_network_packet_data::QuicNetworkPacketData;
use crate::structs::control::ClientAction;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientActionPacket {
    pub action: ClientAction,
    #[serde(default)]
    pub direction: PacketDirection,
    pub occurred_at_ms: u64,
}

impl ClientActionPacket {
    pub fn new(action: ClientAction, direction: PacketDirection) -> Self {
        Self {
            action,
            direction,
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for ClientActionPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ClientAction(c) => Ok(c),
            _ => Err(()),
        }
    }
}
