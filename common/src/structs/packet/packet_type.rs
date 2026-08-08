use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    PlayerPosition,
    ChannelEvent,
    ChatMessage,
    ChatSend,
    Collection,
    Debug,
    PlayerPresence,
    ServerError,
    HealthCheck,
    BedrockEvent,
    PeerPresenceInject,
    PeerPresenceObserved,
    PeerAnnounceInject,
    PeerAnnounceObserved,
    AudioQuery,
    AudioAvailable,
    ClientAction,
    QueryState,
    PlayerPreference,
}
