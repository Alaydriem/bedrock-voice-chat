use std::time::Duration;

use common::game_data::Dimension;
use common::structs::game::BlockCoordinate;
use moka::sync::Cache;

const TTL: Duration = Duration::from_secs(3);
const MAX_CAPACITY: u64 = 64;

pub struct JukeboxBeaconCache {
    inner: Cache<(BlockCoordinate, Dimension), String>,
}

impl JukeboxBeaconCache {
    pub fn new() -> Self {
        Self {
            inner: Cache::builder()
                .time_to_live(TTL)
                .max_capacity(MAX_CAPACITY)
                .build(),
        }
    }

    pub fn observe(&self, position: BlockCoordinate, dimension: Dimension, event_id: &str) {
        self.inner
            .insert((position, dimension), event_id.to_string());
    }

    pub fn resolve_for_eject(
        &self,
        position: BlockCoordinate,
        dimension: Dimension,
    ) -> Option<String> {
        let key = (position, dimension);
        self.inner.get(&key).map(|event_id| {
            self.inner.invalidate(&key);
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
    use common::game_data::Dimension;

    #[test]
    fn observe_then_resolve_returns_event_id_once() {
        let cache = JukeboxBeaconCache::new();
        let pos = BlockCoordinate::new(12, 64, -7);
        cache.observe(pos, Dimension::Overworld, "evt-abc");
        assert_eq!(
            cache
                .resolve_for_eject(pos, Dimension::Overworld)
                .as_deref(),
            Some("evt-abc")
        );
        assert!(cache.resolve_for_eject(pos, Dimension::Overworld).is_none());
    }

    #[test]
    fn unknown_position_returns_none() {
        let cache = JukeboxBeaconCache::new();
        assert!(
            cache
                .resolve_for_eject(BlockCoordinate::new(0, 0, 0), Dimension::Overworld)
                .is_none()
        );
    }

    #[test]
    fn same_coords_different_dimension_do_not_collide() {
        let cache = JukeboxBeaconCache::new();
        let pos = BlockCoordinate::new(100, 64, 100);
        cache.observe(pos, Dimension::Overworld, "evt-overworld");
        cache.observe(pos, Dimension::TheNether, "evt-nether");
        assert_eq!(
            cache
                .resolve_for_eject(pos, Dimension::TheNether)
                .as_deref(),
            Some("evt-nether")
        );
        assert_eq!(
            cache
                .resolve_for_eject(pos, Dimension::Overworld)
                .as_deref(),
            Some("evt-overworld")
        );
    }

    #[test]
    fn floor_handles_negative_coords() {
        use common::structs::game::Coordinate;
        let cache = JukeboxBeaconCache::new();
        let coord = Coordinate {
            x: -0.5,
            y: -10.9,
            z: -100.1,
        };
        let key = BlockCoordinate::from(&coord);
        cache.observe(key, Dimension::Overworld, "evt-neg");
        assert_eq!(
            cache
                .resolve_for_eject(BlockCoordinate::new(-1, -11, -101), Dimension::Overworld)
                .as_deref(),
            Some("evt-neg"),
        );
    }
}
