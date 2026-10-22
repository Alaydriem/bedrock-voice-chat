use std::collections::{HashMap, HashSet};

use common::structs::group_key::SocketId;

/// Which relay socket belongs to which MLS leaf.
///
/// Learned from MLS application messages each member sends after joining, so a binding is
/// authenticated by the member's own leaf key. The server can only lie about a socket closing,
/// which removes a real member — denial of service, not a way in.
#[derive(Default)]
pub struct SocketBindings {
    leaves: HashMap<SocketId, u32>,
    // Sockets whose binding came from their own announcement. The admitter binds a joiner before
    // it announces; that joiner's first announcement must still read as news, or the admitter
    // never answers with its own and the joiner never learns it.
    announced: HashSet<SocketId>,
}

impl SocketBindings {
    /// Binds a socket the member has not announced itself.
    pub fn bind_unannounced(&mut self, socket: SocketId, leaf: u32) {
        self.insert(socket, leaf);
        self.announced.remove(&socket);
    }

    /// Binds a socket from its own announcement. Returns whether this is news: a socket not
    /// announced before, or announced for another leaf.
    pub fn bind_announced(&mut self, socket: SocketId, leaf: u32) -> bool {
        let known = self.announced.contains(&socket) && self.leaf_of(socket) == Some(leaf);
        self.insert(socket, leaf);
        self.announced.insert(socket);
        !known
    }

    pub fn leaf_of(&self, socket: SocketId) -> Option<u32> {
        self.leaves.get(&socket).copied()
    }

    pub fn socket_of(&self, leaf: u32) -> Option<SocketId> {
        self.leaves
            .iter()
            .find(|(_, bound)| **bound == leaf)
            .map(|(socket, _)| *socket)
    }

    pub fn forget(&mut self, socket: SocketId) {
        self.leaves.remove(&socket);
        self.announced.remove(&socket);
    }

    pub fn clear(&mut self) {
        self.leaves.clear();
        self.announced.clear();
    }

    fn insert(&mut self, socket: SocketId, leaf: u32) {
        let replaced: Vec<SocketId> = self
            .leaves
            .iter()
            .filter(|(other, bound)| **bound == leaf && **other != socket)
            .map(|(other, _)| *other)
            .collect();
        for other in replaced {
            self.forget(other);
        }
        self.leaves.insert(socket, leaf);
    }
}
