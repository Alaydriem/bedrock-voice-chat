use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerPositionPacket {
    pub player: crate::PlayerEnum,
}

impl TryFrom<QuicNetworkPacketData> for PlayerPositionPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerPosition(c) => Ok(c),
            _ => Err(()),
        }
    }
}
