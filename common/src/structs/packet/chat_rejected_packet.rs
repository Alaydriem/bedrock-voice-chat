use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::quic_network_packet_data::QuicNetworkPacketData;

/// A line the server refused, returned to the one connection that sent it.
///
/// Carries the text back so the composer can settle the right pending line without the server
/// having to know anything about how the sender rendered it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChatRejectedPacket {
    pub reason: String,
    pub text: String,
}

impl ChatRejectedPacket {
    pub fn new(reason: String, text: String) -> Self {
        Self { reason, text }
    }
}

impl TryFrom<QuicNetworkPacketData> for ChatRejectedPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChatRejected(c) => Ok(c),
            _ => Err(()),
        }
    }
}
