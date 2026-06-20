use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PeerAnnounceInjectPacket {
    pub endpoint: String,
    pub ttl_ms: u32,
}
