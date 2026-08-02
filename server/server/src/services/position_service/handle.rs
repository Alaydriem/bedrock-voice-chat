use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Per-session pseudonyms for the players an observer can see.
///
/// The UI needs a stable key to animate an entry across frames, but shipping
/// gamertags would turn the feed into a player tracker. A random per-session
/// salt yields a handle that is stable while the socket lives and uncorrelated
/// with any other session's view of the same player.
pub struct PositionHandle {
    salt: u64,
}

impl PositionHandle {
    pub fn new_session() -> Self {
        Self {
            salt: rand::random::<u64>(),
        }
    }

    pub fn handle_for(&self, player_name: &str) -> u32 {
        let mut hasher = DefaultHasher::new();
        self.salt.hash(&mut hasher);
        player_name.hash(&mut hasher);
        (hasher.finish() >> 32) as u32
    }
}

impl Default for PositionHandle {
    fn default() -> Self {
        Self::new_session()
    }
}
