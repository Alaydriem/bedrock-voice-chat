use serde::{Deserialize, Serialize};

use super::connection_event_type::ConnectionEventType;
use super::quic_network_packet_data::QuicNetworkPacketData;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerPresenceEvent {
    pub player_name: String,
    pub timestamp: i64, // Unix timestamp in milliseconds
    pub event_type: ConnectionEventType,
}

impl TryFrom<QuicNetworkPacketData> for PlayerPresenceEvent {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerPresence(p) => Ok(p),
            _ => Err(()),
        }
    }
}
