use std::time::Duration;

use moka::sync::Cache;

/// Platform UUIDs already written to a player record, so a roster that repeats several
/// times a second costs one write for each UUID and not one for each tick.
#[derive(Clone)]
pub struct BoundUuidCache {
    cache: Cache<String, ()>,
}

impl BoundUuidCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                // 1 day
                .time_to_live(Duration::from_secs(86400))
                .max_capacity(4096)
                .build(),
        }
    }

    pub fn contains(&self, uuid: &str) -> bool {
        self.cache.get(&uuid.to_ascii_lowercase()).is_some()
    }

    pub fn insert(&self, uuid: &str) {
        self.cache.insert(uuid.to_ascii_lowercase(), ());
    }
}

impl Default for BoundUuidCache {
    fn default() -> Self {
        Self::new()
    }
}
