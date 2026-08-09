use std::time::Duration;

use common::structs::control::QueryState;
use moka::future::Cache;

use super::cache_trait::CacheTrait;

/// Each player's last-reported self-state (`QueryState`), keyed by canonical id.
/// TTL outlives a gaming session (the client seeds it on connect and refreshes on
/// change), so a long-running session never forces a cache rebuild.
#[derive(Clone)]
pub struct PlayerStateCache {
    cache: Cache<String, QueryState>,
}

impl PlayerStateCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(86_400))
                .max_capacity(4096)
                .build(),
        }
    }

    /// Whether any player currently reports an open recording.
    ///
    /// An instant sample. A player who records for five minutes each hour reads as
    /// false in most samples, so this answers whether the feature is in live use, not
    /// how much recording happens here.
    pub fn any_recording(&self) -> bool {
        self.cache.iter().any(|(_, state)| state.recording)
    }
}

impl Default for PlayerStateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheTrait for PlayerStateCache {
    type Key = String;
    type Value = QueryState;

    async fn get(&self, key: &String) -> Option<QueryState> {
        self.cache.get(key).await
    }

    async fn set(&self, key: String, value: QueryState) {
        self.cache.insert(key, value).await;
    }

    async fn delete(&self, key: &String) {
        self.cache.invalidate(key).await;
    }
}
