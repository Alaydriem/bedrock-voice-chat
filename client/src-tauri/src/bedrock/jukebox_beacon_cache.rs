use std::time::Duration;

use common::structs::game::Coordinate;
use moka::sync::Cache;

const TTL: Duration = Duration::from_secs(3);
const MAX_CAPACITY: u64 = 64;
const PENDING_INSERT_TTL: Duration = Duration::from_secs(5);

pub struct JukeboxBeaconCache {
    inner: Cache<(i32, i32, i32), String>,
    pending_insert_update: Cache<(i32, i32, i32), ()>,
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

    pub fn observe(&self, position: &Coordinate, event_id: &str) {
        self.inner
            .insert(Self::key_from_coord(position), event_id.to_string());
    }

    pub fn note_insert_pending(&self, position: &Coordinate) {
        self.pending_insert_update
            .insert(Self::key_from_coord(position), ());
    }

    pub fn process_update_block(&self, block_key: (i32, i32, i32)) -> Option<String> {
        if self.pending_insert_update.get(&block_key).is_some() {
            self.pending_insert_update.invalidate(&block_key);
            return None;
        }
        self.inner.get(&block_key).map(|event_id| {
            self.inner.invalidate(&block_key);
            event_id
        })
    }

    fn key_from_coord(c: &Coordinate) -> (i32, i32, i32) {
        (c.x.floor() as i32, c.y.floor() as i32, c.z.floor() as i32)
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
        let pos = Coordinate { x: 12.0, y: 64.0, z: -7.0 };
        cache.observe(&pos, "evt-abc");
        let result = cache.process_update_block((12, 64, -7));
        assert_eq!(result.as_deref(), Some("evt-abc"));
        assert!(cache.process_update_block((12, 64, -7)).is_none());
    }

    #[test]
    fn note_insert_pending_consumes_first_update_block() {
        let cache = JukeboxBeaconCache::new();
        let pos = Coordinate { x: 5.0, y: 70.0, z: 5.0 };
        cache.note_insert_pending(&pos);
        cache.observe(&pos, "evt-xyz");
        assert!(cache.process_update_block((5, 70, 5)).is_none());
        let second = cache.process_update_block((5, 70, 5));
        assert_eq!(second.as_deref(), Some("evt-xyz"));
    }

    #[test]
    fn unknown_position_returns_none() {
        let cache = JukeboxBeaconCache::new();
        assert!(cache.process_update_block((0, 0, 0)).is_none());
    }

    #[test]
    fn floor_handles_negative_coords() {
        let cache = JukeboxBeaconCache::new();
        let pos = Coordinate { x: -0.5, y: -10.9, z: -100.1 };
        cache.observe(&pos, "evt-neg");
        assert_eq!(
            cache.process_update_block((-1, -11, -101)).as_deref(),
            Some("evt-neg"),
        );
    }
}
