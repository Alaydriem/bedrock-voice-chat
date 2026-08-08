use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;

/// One mod's chat connection.
pub struct ChatSocket {
    pub world_name: String,
    pub tx: mpsc::Sender<String>,
    /// Every id this room spans, shared by all of its entries.
    ///
    /// Paper and Fabric mint an id per dimension while chat is server-wide, so one room is
    /// several ids. Holding the whole set on each entry is what lets a lookup by any one id
    /// answer for the room rather than for the dimension.
    pub room: Arc<Vec<String>>,
}

/// One socket per world.
///
/// A mod process owns exactly one world, so this is also one socket per mod process — which
/// is what makes displacement on re-registration the correct behaviour rather than a policy.
pub struct ChatSocketRegistry {
    sockets: DashMap<String, ChatSocket>,
}

impl ChatSocketRegistry {
    pub fn new() -> Self {
        Self {
            sockets: DashMap::new(),
        }
    }

    /// Returns the displaced sender, if any, so the caller can close that socket.
    ///
    /// A mod restart whose previous socket has not yet timed out would otherwise leave two
    /// registered for one world: every `say` pushed twice, every line reported twice.
    pub fn register(
        &self,
        world_uuid: String,
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Option<mpsc::Sender<String>> {
        let room = Arc::new(vec![world_uuid.clone()]);
        self.sockets
            .insert(
                world_uuid,
                ChatSocket {
                    world_name,
                    tx,
                    room,
                },
            )
            .map(|previous| previous.tx)
    }

    /// Registers one socket under every id its room spans.
    ///
    /// Returns any displaced senders so the caller can close those sockets: two registrations
    /// for one id would double every message.
    pub fn register_room(
        &self,
        worlds: &[String],
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Vec<mpsc::Sender<String>> {
        let room = Arc::new(worlds.to_vec());
        let mut displaced = Vec::new();

        for world_uuid in worlds {
            if let Some(previous) = self.sockets.insert(
                world_uuid.clone(),
                ChatSocket {
                    world_name: world_name.clone(),
                    tx: tx.clone(),
                    room: Arc::clone(&room),
                },
            ) {
                displaced.push(previous.tx);
            }
        }

        displaced
    }

    /// Every id sharing a room with this one, including itself.
    pub fn room(&self, world_uuid: &str) -> Option<Arc<Vec<String>>> {
        self.sockets.get(world_uuid).map(|s| Arc::clone(&s.room))
    }

    pub fn unregister(&self, world_uuid: &str) {
        self.sockets.remove(world_uuid);
    }

    pub fn contains(&self, world_uuid: &str) -> bool {
        self.sockets.contains_key(world_uuid)
    }

    pub fn sender(&self, world_uuid: &str) -> Option<mpsc::Sender<String>> {
        self.sockets.get(world_uuid).map(|s| s.tx.clone())
    }

    pub fn world_name(&self, world_uuid: &str) -> Option<String> {
        self.sockets.get(world_uuid).map(|s| s.world_name.clone())
    }
}

impl Default for ChatSocketRegistry {
    fn default() -> Self {
        Self::new()
    }
}
