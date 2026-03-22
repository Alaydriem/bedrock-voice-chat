use serde::{Deserialize, Serialize};

use crate::structs::channel::ChannelEvents;
use super::quic_network_packet_data::QuicNetworkPacketData;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelEventPacket {
    pub event: ChannelEvents,
    pub name: String,
    pub channel: String,
    pub channel_name: Option<String>,
    pub creator: Option<String>,
    pub timestamp: Option<i64>,
}

impl ChannelEventPacket {
    pub fn new(event: ChannelEvents, player_name: String, channel_id: String) -> Self {
        Self {
            event,
            name: player_name,
            channel: channel_id,
            channel_name: None,
            creator: None,
            timestamp: None,
        }
    }

    pub fn new_full(
        event: ChannelEvents,
        player_name: String,
        channel_id: String,
        channel_name: Option<String>,
        creator: Option<String>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        Self {
            event,
            name: player_name,
            channel: channel_id,
            channel_name,
            creator,
            timestamp: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
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
