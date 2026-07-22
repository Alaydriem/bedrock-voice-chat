use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PacketDirection {
    ServerBound,
    ClientBound,
}

impl Default for PacketDirection {
    fn default() -> Self {
        PacketDirection::ServerBound
    }
}
