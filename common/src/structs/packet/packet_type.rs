use serde::{Deserialize, Serialize};

/// Postcard encodes a variant as its **index**, so this list is a wire format. Adding a
/// variant anywhere but the end shifts every later discriminant and silently mis-decodes
/// every packet after it — append only.
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
    ChatRejected,
}
