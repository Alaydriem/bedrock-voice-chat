use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    PlayerPosition,
    ChannelEvent,
    Collection,
    Debug,
    PlayerPresence,
    ServerError,
    HealthCheck,
    BedrockEvent,
    PeerPresenceInject,
    PeerPresenceObserved,
    AudioQuery,
    AudioAvailable,
}
