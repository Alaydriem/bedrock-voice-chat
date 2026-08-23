use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::quic_network_packet_data::QuicNetworkPacketData;

/// A line composed in the app, travelling client to server.
///
/// Carries no author. The sender is the mTLS identity on the connection, resolved server-side;
/// trusting a body field here would let any client post as any player.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChatSendPacket {
    /// Absent on the no-net path, which has no world key and never reaches the server.
    pub world_uuid: Option<String>,
    pub text: String,
}

impl ChatSendPacket {
    pub fn new(world_uuid: Option<String>, text: String) -> Self {
        Self { world_uuid, text }
    }
}

impl TryFrom<QuicNetworkPacketData> for ChatSendPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChatSend(c) => Ok(c),
            _ => Err(()),
        }
    }
}
