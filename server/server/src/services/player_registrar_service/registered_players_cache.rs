use std::time::Duration;

use moka::sync::Cache;

/// Cache of registered player names to avoid repeated database queries
#[derive(Clone)]
pub struct RegisteredPlayersCache {
    cache: Cache<String, bool>,
}

impl RegisteredPlayersCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400)) // 1 day
                .max_capacity(512)
                .build(),
        }
    }

    pub fn contains(&self, player_name: &str) -> bool {
        self.cache.get(player_name).is_some()
    }

    pub fn insert(&self, player_name: String) {
        self.cache.insert(player_name, true);
    }
}

impl Default for RegisteredPlayersCache {
    fn default() -> Self {
        Self::new()
    }
}
