use std::time::Instant;

use common::structs::group_key::SocketId;

/// A join request on this member's screen, or waiting for room on it.
pub struct OpenRequest {
    pub joiner: SocketId,
    /// The identity the server authenticated for the joiner's socket.
    pub identity: String,
    pub commitment: [u8; 32],
    pub received_at: Instant,
    /// This member's live check of the joiner.
    pub check_id: Option<[u8; 16]>,
    /// Another member is checking the joiner.
    pub elsewhere: bool,
    /// Admitted; waiting for the Add and the Welcome.
    pub admitting: bool,
    /// A card was raised for it.
    pub shown: bool,
}
