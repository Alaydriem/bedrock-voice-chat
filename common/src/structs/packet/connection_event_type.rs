use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum ConnectionEventType {
    Connected,
    Disconnected,
}
