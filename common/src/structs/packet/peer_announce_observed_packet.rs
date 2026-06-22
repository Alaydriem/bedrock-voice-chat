use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PeerAnnounceObservedPacket {
    pub hashed_world: String,
    pub endpoint: String,
}
