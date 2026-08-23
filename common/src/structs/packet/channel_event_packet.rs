use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;
use crate::structs::channel::ChannelEvents;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelEventPacket {
    pub event: ChannelEvents,
    /// The player the membership change applies to, which is legitimately somebody other
    /// than the sender when the channel API acts on a player's behalf.
    pub name: crate::PlayerIdentity,
    pub channel: String,
    pub channel_name: Option<String>,
    /// `None` only when the channel is already gone and there is no owner left to name.
    pub creator: Option<crate::PlayerIdentity>,
    pub timestamp: Option<i64>,
}

impl ChannelEventPacket {
    pub fn new(
        event: ChannelEvents,
        name: crate::PlayerIdentity,
        channel: String,
        channel_name: Option<String>,
        creator: Option<crate::PlayerIdentity>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        Self {
            event,
            name,
            channel,
            channel_name,
            creator,
            timestamp: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
            ),
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for ChannelEventPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChannelEvent(c) => Ok(c),
            _ => Err(()),
        }
    }
}
