use std::time::Duration;

use common::PlayerEnum;
use moka::sync::Cache;
use parking_lot::RwLock;

pub struct BedrockPlayerStateCache {
    cache: Cache<String, PlayerEnum>,
    local_gamertag: RwLock<Option<String>>,
}

impl BedrockPlayerStateCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .max_capacity(64)
                .build(),
            local_gamertag: RwLock::new(None),
        }
    }

    pub fn set(&self, gamertag: &str, player: PlayerEnum) {
        self.cache.insert(gamertag.to_string(), player);
    }

    pub fn get(&self, gamertag: &str) -> Option<PlayerEnum> {
        self.cache.get(gamertag)
    }

    pub fn get_local_player(&self) -> Option<PlayerEnum> {
        let tag = self.local_gamertag.read();
        tag.as_ref().and_then(|t| self.get(t))
    }

    pub fn set_local_gamertag(&self, gamertag: String) {
        *self.local_gamertag.write() = Some(gamertag);
    }

    pub fn clear(&self) {
        self.cache.invalidate_all();
        *self.local_gamertag.write() = None;
    }
}
