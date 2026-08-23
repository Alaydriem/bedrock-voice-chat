use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

use super::chat_origin::ChatOrigin;
use super::quic_network_packet_data::QuicNetworkPacketData;

/// One line of in-game chat, travelling server to client.
///
/// Nothing about this is stored. The app holds a bounded in-memory ring and the server holds
/// none at all, so the only history that exists is the one a client accumulated while connected.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChatMessagePacket {
    /// Absent for a server-authored line.
    pub author: Option<String>,
    pub text: String,
    /// Absent on the no-net path, where the proxy session is the world and there is no key.
    pub world_uuid: Option<String>,
    pub origin: ChatOrigin,
    pub occurred_at_ms: u64,
}

impl ChatMessagePacket {
    pub fn new(
        author: Option<String>,
        text: String,
        world_uuid: Option<String>,
        origin: ChatOrigin,
    ) -> Self {
        Self {
            author,
            text,
            world_uuid,
            origin,
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for ChatMessagePacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChatMessage(c) => Ok(c),
            _ => Err(()),
        }
    }
}
