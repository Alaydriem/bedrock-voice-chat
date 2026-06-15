use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PeerPresenceInjectPacket {
    pub token: String,
    pub ttl_ms: u32,
}
