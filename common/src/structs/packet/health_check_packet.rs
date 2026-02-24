use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;

/// Health check packet for connection monitoring
/// Empty packet - just the type marker is enough for ping/pong behavior
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckPacket;

impl TryFrom<QuicNetworkPacketData> for HealthCheckPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::HealthCheck(h) => Ok(h),
            _ => Err(()),
        }
    }
}
