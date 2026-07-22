use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::quic_network_packet_data::QuicNetworkPacketData;
use crate::structs::control::PlayerPreference;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerPreferencePacket {
    pub preference: PlayerPreference,
    pub occurred_at_ms: u64,
}

impl PlayerPreferencePacket {
    pub fn new(preference: PlayerPreference) -> Self {
        Self {
            preference,
            occurred_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for PlayerPreferencePacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerPreference(c) => Ok(c),
            _ => Err(()),
        }
    }
}
