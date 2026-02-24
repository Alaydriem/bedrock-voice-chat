use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;

/// Debug Packet
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket {
    pub owner: String,
    pub version: String,
    pub timestamp: u64,
}

impl TryFrom<QuicNetworkPacketData> for DebugPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::Debug(c) => Ok(c),
            _ => Err(()),
        }
    }
}
