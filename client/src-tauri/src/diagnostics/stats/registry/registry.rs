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

    /// Drops both routes for one sink key.
    ///
    /// For a speaker known to have stopped for good rather than merely gone quiet. A jukebox key
    /// carries its playback's event id, so a second disc in the same block is a different speaker
    /// here and the old key is never written again.
    pub fn unregister(&self, sink_key: &str) {
        self.entries
            .invalidate(&(sink_key.to_string(), PeerRoute::Normal));
        self.entries
            .invalidate(&(sink_key.to_string(), PeerRoute::Spatial));
    }

    /// Every jukebox sink key with the frames it has received.
    ///
    /// Keyed by sink rather than by display name, because the caller compares one reading against
    /// the next to decide whether a playback has stopped, and `peers()` merges names and hides
    /// anything idle — which is precisely the state being looked for. The two routes fold to the
    /// higher count so a stalled second route cannot make a live sink look quiet.
    pub fn jukebox_frame_counts(&self) -> Vec<(String, u64)> {
        let mut counts: Vec<(String, u64)> = Vec::new();

        for (key, stats) in self.entries.iter() {
            let (sink_key, _) = &*key;
            if !sink_key.starts_with(common::consts::audio::JUKEBOX_PLAYER_PREFIX) {
                continue;
            }

            let received = stats.frames_received();
            match counts.iter_mut().find(|(existing, _)| existing == sink_key) {
                Some((_, highest)) => *highest = (*highest).max(received),
                None => counts.push((sink_key.clone(), received)),
            }
        }

        counts.sort_by(|a, b| a.0.cmp(&b.0));
        counts
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
