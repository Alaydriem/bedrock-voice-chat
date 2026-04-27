use std::sync::Mutex;
use std::time::Duration;

use common::PlayerEnum;
use moka::sync::Cache;

pub struct BedrockPositionCache {
    cache: Cache<String, PlayerEnum>,
    local_gamertag: Mutex<Option<String>>,
}

impl BedrockPositionCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .max_capacity(64)
                .build(),
            local_gamertag: Mutex::new(None),
        }
    }

    pub fn set(&self, gamertag: &str, player: PlayerEnum) {
        self.cache.insert(gamertag.to_string(), player);
    }

    pub fn get(&self, gamertag: &str) -> Option<PlayerEnum> {
        self.cache.get(gamertag)
    }

    pub fn get_local_player(&self) -> Option<PlayerEnum> {
        self.local_gamertag
            .lock()
            .ok()
            .and_then(|tag| tag.as_ref().and_then(|t| self.get(t)))
    }

    pub fn set_local_gamertag(&self, gamertag: String) {
        if let Ok(mut tag) = self.local_gamertag.lock() {
            *tag = Some(gamertag);
        }
    }

    pub fn clear(&self) {
        self.cache.invalidate_all();
        if let Ok(mut tag) = self.local_gamertag.lock() {
            *tag = None;
        }
    }
}
