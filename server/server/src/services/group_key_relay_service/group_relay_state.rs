use common::structs::group_key::{GroupKeyFrame, SocketId};
use tokio::sync::mpsc::error::TrySendError;

use super::socket_entry::SocketEntry;

/// One protected group's sockets, in the order every frame is delivered to them.
#[derive(Default)]
pub struct GroupRelayState {
    pub sockets: Vec<SocketEntry>,
    any_cursor: usize,
}

impl GroupRelayState {
    pub fn find(&mut self, id: SocketId) -> Option<&mut SocketEntry> {
        self.sockets.iter_mut().find(|s| s.id == id)
    }

    /// The next announced socket other than `from`, rotating so admission load spreads across
    /// members.
    pub fn pick_other(&mut self, from: SocketId) -> Option<SocketId> {
        let candidates: Vec<SocketId> = self
            .sockets
            .iter()
            .filter(|s| s.announced && s.id != from)
            .map(|s| s.id)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = candidates[self.any_cursor % candidates.len()];
        self.any_cursor = self.any_cursor.wrapping_add(1);
        Some(pick)
    }

    /// Removes a socket and tells the rest it is gone. Dropping its sender ends its queue.
    pub fn remove(&mut self, id: SocketId) -> bool {
        let before = self.sockets.len();
        self.sockets.retain(|s| s.id != id);
        if self.sockets.len() == before {
            return false;
        }
        self.deliver_all(GroupKeyFrame::Gone { member: id }, None);
        true
    }

    /// Delivers to one socket. A full or closed queue removes that socket, so a slow member
    /// cannot stall the group.
    pub fn deliver(&mut self, to: SocketId, frame: GroupKeyFrame) {
        let failed = match self.sockets.iter().find(|s| s.id == to) {
            Some(socket) => matches!(
                socket.tx.try_send(frame),
                Err(TrySendError::Full(_)) | Err(TrySendError::Closed(_))
            ),
            None => false,
        };
        if failed {
            self.remove(to);
        }
    }

    /// Delivers to every socket except `skip`.
    pub fn deliver_all(&mut self, frame: GroupKeyFrame, skip: Option<SocketId>) {
        let targets: Vec<SocketId> = self
            .sockets
            .iter()
            .map(|s| s.id)
            .filter(|id| Some(*id) != skip)
            .collect();
        for id in targets {
            self.deliver(id, frame.clone());
        }
    }
}
