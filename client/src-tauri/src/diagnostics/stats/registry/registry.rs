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
    entries: Cache<(Vec<u8>, PeerRoute), Arc<PlayerReceiveStats>>,
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

    pub fn register(&self, client_id: Vec<u8>, route: PeerRoute, stats: Arc<PlayerReceiveStats>) {
        self.entries.insert((client_id, route), stats);
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
}
