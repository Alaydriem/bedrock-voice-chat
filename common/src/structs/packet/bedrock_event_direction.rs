use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum BedrockEventDirection {
    ServerBound,
    ClientBound,
    PeerBound,
}

impl Default for BedrockEventDirection {
    fn default() -> Self {
        Self::ServerBound
    }
}
