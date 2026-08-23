use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;

/// The client's first datagram. Opens the stream and declares the protocol version.
///
/// Carries no identity: the server takes that from the certificate it authenticated at
/// accept, so a field here would be a claim a client makes about itself that nothing reads.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket {
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
