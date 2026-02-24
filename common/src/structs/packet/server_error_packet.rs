use serde::{Deserialize, Serialize};

use super::server_error_type::ServerErrorType;
use super::quic_network_packet_data::QuicNetworkPacketData;

/// Server Error Packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerErrorPacket {
    pub error_type: ServerErrorType,
    pub message: String,
}

impl TryFrom<QuicNetworkPacketData> for ServerErrorPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ServerError(s) => Ok(s),
            _ => Err(()),
        }
    }
}
