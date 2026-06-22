use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PeerPresenceObservedPacket {
    pub token: String,
}
