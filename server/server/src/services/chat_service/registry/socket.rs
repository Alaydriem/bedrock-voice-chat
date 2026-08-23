use std::sync::Arc;

use tokio::sync::mpsc;

/// One mod's chat connection.
pub struct ChatSocket {
    /// Distinguishes this connection from any other that has held the same world id.
    ///
    /// Every registry write is keyed by world, so without it a socket that is displaced and
    /// dies later removes whatever entry currently holds its key — the live socket's. Chat
    /// then stops with nothing logged and no frame refused.
    pub id: u64,
    pub world_name: String,
    pub tx: mpsc::Sender<String>,
    /// Every id this room spans, shared by all of its entries.
    ///
    /// Paper and Fabric mint an id per dimension while chat is server-wide, so one room is
    /// several ids. Holding the whole set on each entry is what lets a lookup by any one id
    /// answer for the room rather than for the dimension.
    pub room: Arc<Vec<String>>,
}
