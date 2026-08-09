use std::sync::Arc;

use dashmap::DashMap;
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
        id: u64,
        world_uuid: String,
        world_name: String,
        tx: mpsc::Sender<String>,
    ) -> Option<mpsc::Sender<String>> {
        let room = Arc::new(vec![world_uuid.clone()]);
        self.sockets
            .insert(
                world_uuid,
                ChatSocket {
                    id,
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
        id: u64,
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
                    id,
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

    /// Every registered room, as (canonical id, world name).
    ///
    /// Deduplicated by room rather than by entry: a room spanning three dimensions is one
    /// place to talk, not three.
    pub fn rooms(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();

        for entry in self.sockets.iter() {
            let Some(canonical) = entry.room.first() else {
                continue;
            };
            if out.iter().any(|(id, _)| id == canonical) {
                continue;
            }
            out.push((canonical.clone(), entry.world_name.clone()));
        }

        out
    }

    /// Releases this id only if `id` still holds it.
    ///
    /// A displaced socket keeps running until its own transport notices, and it tears down
    /// under the ids it registered. Removing by id alone would take the live socket's entry
    /// with it.
    pub fn unregister(&self, world_uuid: &str, id: u64) {
        self.sockets.remove_if(world_uuid, |_, socket| socket.id == id);
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
