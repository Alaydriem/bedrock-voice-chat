use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::{GroupKeyRecipient, SocketId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKeyFrame {
    /// Client to server: fan out to every other socket on the group.
    Broadcast(Bytes),
    /// Client to server: an MLS commit built on `epoch`. Server to client: an accepted commit,
    /// delivered to every socket including its sender.
    Commit { epoch: u64, body: Bytes },
    /// Client to server: deliver to one socket.
    Direct { to: GroupKeyRecipient, body: Bytes },
    /// Server to client: the commit lost; `epoch` is the one the next commit must build on.
    Stale { epoch: u64 },
    /// Server to client: a socket on the group closed.
    Gone { member: SocketId },
    /// Server to client: a `Direct` or `Broadcast` from another socket.
    Delivered { from: SocketId, body: Bytes },
    /// Server to client, once, when the socket opens: this socket's own id.
    Welcome { socket: SocketId },
    /// Server to client: a `Direct { to: Members }` from another socket, with the canonical
    /// identity the server authenticated for that socket. The identity is the server's word, not
    /// proof; it stops one client from asking in another player's name.
    Asked {
        from: SocketId,
        identity: String,
        body: Bytes,
    },
}

impl GroupKeyFrame {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        Ok(postcard::to_stdvec(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
}
