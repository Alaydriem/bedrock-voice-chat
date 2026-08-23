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
