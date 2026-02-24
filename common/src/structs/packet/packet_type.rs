use serde::{Deserialize, Serialize};

/// The packet type
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    ChannelEvent,
    Collection,
    Debug,
    PlayerPresence,
    ServerError,
    HealthCheck,
}
