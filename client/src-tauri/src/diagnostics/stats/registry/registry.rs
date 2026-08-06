use std::sync::Arc;
use std::time::Duration;

use common::structs::metrics::PeerDiagnostics;
use moka::sync::Cache;

use super::PeerRoute;
use crate::diagnostics::PlayerReceiveStats;


// The per-speaker counter registry.
//
// Counters are created where a jitter buffer is created and registered here, because the buffer
// itself is moved into rodio's graph and cannot be read afterwards. Reading through this
// registry also means a diagnostic never has to lock the audio manager, which is contended with
// playback.
//
// Entries expire on the same schedule as the sinks they describe, so an entry disappearing means
// a speaker stopped being heard rather than a bookkeeping mistake.
#[derive(Debug)]
pub struct PeerRegistry {
    entries: Cache<(String, PeerRoute), Arc<PlayerReceiveStats>>,
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerRegistry {
    // The TTL matches the sink cache this mirrors. The capacity is twice its hundred-speaker cap
    // because a speaker heard both normally and spatially occupies two entries here and one there.
    const TTL: Duration = Duration::from_secs(15 * 60);
    const MAX_CAPACITY: u64 = 200;

    pub fn new() -> Self {
        Self {
            entries: Cache::builder()
                .time_to_live(Self::TTL)
                .max_capacity(Self::MAX_CAPACITY)
                .build(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Registers a receive-stats record for one speaker on one route.
    ///
    /// `sink_key` matches the mixer's sink key, so a diagnostics row and the sink it
    /// describes cannot drift apart.
    pub fn register(&self, sink_key: String, route: PeerRoute, stats: Arc<PlayerReceiveStats>) {
        self.entries.insert((sink_key, route), stats);
    }

    // One record per speaker, with both routes folded together, sorted by name so a report and
    // a log line list speakers in a stable order.
    //
    // Speakers with no traffic are omitted: an idle eight-player server must not emit eight rows
    // of zeros every interval.
    pub fn peers(&self) -> Vec<PeerDiagnostics> {
        let mut by_name: Vec<(String, PeerDiagnostics)> = Vec::new();

        for (_, stats) in self.entries.iter() {
            if stats.is_idle() {
                continue;
            }

            let name = stats.name().to_string();
            match by_name.iter_mut().find(|(existing, _)| existing == &name) {
                Some((_, base)) => stats.merge_into(base),
                None => by_name.push((name, stats.to_diagnostics())),
            }
        }

        by_name.sort_by(|a, b| a.0.cmp(&b.0));
        by_name.into_iter().map(|(_, d)| d).collect()
    }

    pub fn peer_count(&self) -> u32 {
        self.peers().len() as u32
    }

    // Zeroes every speaker's counters in place.
    //
    // In place rather than by invalidating the cache: the entries are the same `Arc`s the live
    // jitter buffers write into, and those buffers are inside rodio's graph with no handle left
    // to re-register them. Dropping the entries would leave every currently-heard speaker
    // unreportable for the rest of their sink's life.
    pub fn reset(&self) {
        for (_, stats) in self.entries.iter() {
            stats.reset_counters();
        }
    }
}
