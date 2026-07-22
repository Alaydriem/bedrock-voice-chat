use std::sync::Arc;
use std::time::Duration;

use common::PlayerEnum;
use moka::future::Cache;

use super::cache_trait::CacheTrait;

/// The position/identity cache, keyed by bare gamertag. Short TTL — it mirrors
/// live in-game presence and is refreshed continuously by position packets.
///
/// Beyond the uniform `CacheTrait`, it exposes the raw moka handle for the audio
/// hot path (`route_audio_frame`, `AudioFramePacket::update_coordinates`) and
/// iteration (relay-world discovery), which need direct access for per-frame work.
#[derive(Clone)]
pub struct PlayerCache {
    cache: Arc<Cache<String, PlayerEnum>>,
}

impl PlayerCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(
                Cache::builder()
                    .time_to_live(Duration::from_secs(300))
                    .max_capacity(256)
                    .build(),
            ),
        }
    }

    /// Cloned handle for the audio hot path (per-frame lookups in
    /// `route_audio_frame` / `update_coordinates`).
    pub fn inner_arc(&self) -> Arc<Cache<String, PlayerEnum>> {
        self.cache.clone()
    }

    /// Distinct `relay_world_uuid`s of players currently cached — backs the relay
    /// `ActiveWorldsSource` so the discovery task advertises only worlds this
    /// server is actively hosting clients in. Lock-light snapshot iteration; never
    /// on the audio hot path.
    pub fn relay_worlds(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut seen: HashSet<String> = HashSet::new();
        for (_, player) in self.cache.iter() {
            if let Some(mc) = player.as_minecraft() {
                if let Some(world) = &mc.relay_world_uuid {
                    seen.insert(world.clone());
                }
            }
        }
        seen.into_iter().collect()
    }
}

impl Default for PlayerCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheTrait for PlayerCache {
    type Key = String;
    type Value = PlayerEnum;

    async fn get(&self, key: &String) -> Option<PlayerEnum> {
        self.cache.get(key).await
    }

    async fn set(&self, key: String, value: PlayerEnum) {
        self.cache.insert(key, value).await;
    }

    async fn delete(&self, key: &String) {
        self.cache.invalidate(key).await;
    }
}
