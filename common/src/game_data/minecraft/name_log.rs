use std::sync::LazyLock;
use std::time::Duration;

use moka::sync::Cache;

use super::DimensionHash;
use crate::game_data::BoundedLogText;

// Hashes already logged. Bounded so a stream of distinct names cannot grow it.
static SEEN: LazyLock<Cache<DimensionHash, ()>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(DimensionNameLog::CAPACITY)
        .time_to_live(DimensionNameLog::TTL)
        .build()
});

/// Logs the readable name behind a dimension hash the first time it is seen, so an operator
/// can map the hash in later log lines and errors back to a dimension.
pub struct DimensionNameLog;

impl DimensionNameLog {
    const CAPACITY: u64 = 1024;
    const TTL: Duration = Duration::from_secs(3600);

    pub fn observe(raw: &str, hash: DimensionHash) {
        if SEEN.contains_key(&hash) {
            return;
        }
        SEEN.insert(hash, ());

        crate::curia::info!("first sighting of a custom dimension", {
            "dimension": BoundedLogText::of(raw),
            "dimension_hash": hash.to_hex(),
        });
    }
}
