use std::time::Duration;

use common::structs::game::BlockCoordinate;
use moka::sync::Cache;

const TTL: Duration = Duration::from_secs(3);
const MAX_CAPACITY: u64 = 64;
const PENDING_INSERT_TTL: Duration = Duration::from_secs(5);

pub struct JukeboxBeaconCache {
    inner: Cache<BlockCoordinate, String>,
    pending_insert_update: Cache<BlockCoordinate, ()>,
}

impl JukeboxBeaconCache {
    pub fn new() -> Self {
        Self {
            inner: Cache::builder()
                .time_to_live(TTL)
                .max_capacity(MAX_CAPACITY)
                .build(),
            pending_insert_update: Cache::builder()
                .time_to_live(PENDING_INSERT_TTL)
                .max_capacity(MAX_CAPACITY)
                .build(),
        }
    }

    pub fn observe(&self, position: BlockCoordinate, event_id: &str) {
        self.inner.insert(position, event_id.to_string());
    }

    pub fn note_insert_pending(&self, position: BlockCoordinate) {
        self.pending_insert_update.insert(position, ());
    }

    pub fn process_update_block(&self, position: BlockCoordinate) -> Option<String> {
        if self.pending_insert_update.get(&position).is_some() {
            self.pending_insert_update.invalidate(&position);
            return None;
        }
        self.inner.get(&position).map(|event_id| {
            self.inner.invalidate(&position);
            event_id
        })
    }
}

impl Default for JukeboxBeaconCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_then_process_first_update_block_emits_eject() {
        let cache = JukeboxBeaconCache::new();
        let pos = BlockCoordinate::new(12, 64, -7);
        cache.observe(pos, "evt-abc");
        let result = cache.process_update_block(pos);
        assert_eq!(result.as_deref(), Some("evt-abc"));
        assert!(cache.process_update_block(pos).is_none());
    }

    #[test]
    fn note_insert_pending_consumes_first_update_block() {
        let cache = JukeboxBeaconCache::new();
        let pos = BlockCoordinate::new(5, 70, 5);
        cache.note_insert_pending(pos);
        cache.observe(pos, "evt-xyz");
        assert!(cache.process_update_block(pos).is_none());
        let second = cache.process_update_block(pos);
        assert_eq!(second.as_deref(), Some("evt-xyz"));
    }

    #[test]
    fn unknown_position_returns_none() {
        let cache = JukeboxBeaconCache::new();
        assert!(
            cache
                .process_update_block(BlockCoordinate::new(0, 0, 0))
                .is_none()
        );
    }

    #[test]
    fn floor_handles_negative_coords() {
        use common::structs::game::Coordinate;
        let cache = JukeboxBeaconCache::new();
        let coord = Coordinate { x: -0.5, y: -10.9, z: -100.1 };
        let key = BlockCoordinate::from(&coord);
        cache.observe(key, "evt-neg");
        assert_eq!(
            cache
                .process_update_block(BlockCoordinate::new(-1, -11, -101))
                .as_deref(),
            Some("evt-neg"),
        );
    }
}
