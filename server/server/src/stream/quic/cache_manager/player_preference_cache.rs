use std::time::Duration;

use common::structs::control::{PlayerPreference, PreferenceKey};
use moka::future::Cache;

use super::cache_trait::CacheTrait;

/// Per-player local preferences, keyed by `(owner, target)`. `set` sanitizes the
/// client-supplied gain at the boundary. Reads are SCOPED to the players the panel
/// is showing (`get_scoped`), never the whole store, and an owner's entries are
/// evicted on their disconnect.
#[derive(Clone)]
pub struct PlayerPreferenceCache {
    cache: Cache<PreferenceKey, PlayerPreference>,
}

impl PlayerPreferenceCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(86_400))
                .max_capacity(65_536)
                .build(),
        }
    }

    /// The owner's preferences for exactly the requested targets.
    pub async fn get_scoped(&self, owner: &str, targets: &[String]) -> Vec<PlayerPreference> {
        let mut out = Vec::with_capacity(targets.len());
        for t in targets {
            if let Some(p) = self.get(&PreferenceKey::new(owner, t.clone())).await {
                out.push(p);
            }
        }
        out
    }

    /// Invalidate every preference owned by `owner` (called on disconnect).
    pub async fn evict_owner(&self, owner: &str) {
        let owner = owner.to_string();
        let _ = self.cache.invalidate_entries_if(move |k, _| k.owner == owner);
    }
}

impl Default for PlayerPreferenceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheTrait for PlayerPreferenceCache {
    type Key = PreferenceKey;
    type Value = PlayerPreference;

    async fn get(&self, key: &PreferenceKey) -> Option<PlayerPreference> {
        self.cache.get(key).await
    }

    async fn set(&self, key: PreferenceKey, mut value: PlayerPreference) {
        // Sanitize the client-supplied gain before it can be served back out.
        value.volume = if value.volume.is_finite() {
            value.volume.clamp(0.0, 2.0)
        } else {
            0.0
        };
        self.cache.insert(key, value).await;
    }

    async fn delete(&self, key: &PreferenceKey) {
        self.cache.invalidate(key).await;
    }
}
