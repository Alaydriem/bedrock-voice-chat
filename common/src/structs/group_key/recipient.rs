use serde::{Deserialize, Serialize};

use super::SocketId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKeyRecipient {
    /// Any member's socket other than the sender's, chosen by the server.
    Any,
    Socket(SocketId),
}
