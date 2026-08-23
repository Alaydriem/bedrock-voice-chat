mod route;

pub use route::PeerRoute;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::structs::metrics::PeerDiagnostics;
use moka::sync::Cache;

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
    // When a jukebox frame last arrived, in milliseconds since the epoch. Zero means none has.
    //
    // Separate from the per-sink counters because those are bumped by the jitter buffer, which a
    // muted frame never reaches. A surface that asked them whether music was playing would go
    // quiet the moment the user muted, leaving no way to tell a mute from a disc that ended.
    jukebox_last_frame_ms: AtomicU64,
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

    // How stale an arrival may be and still read as playing. Comfortably longer than the gap
    // between frames, short enough that a stopped disc clears within a poll or two.
    pub const JUKEBOX_PLAYING_WINDOW: Duration = Duration::from_secs(2);

    pub fn new() -> Self {
        Self {
            entries: Cache::builder()
                .time_to_live(Self::TTL)
                .max_capacity(Self::MAX_CAPACITY)
                .build(),
            jukebox_last_frame_ms: AtomicU64::new(0),
        }
    }

    /// Records that a jukebox frame arrived, whatever happens to it afterwards.
    pub fn note_jukebox_frame(&self) {
        self.note_jukebox_frame_at(Self::now_ms());
    }

    /// Whether a jukebox frame has arrived within `within`.
    pub fn jukebox_playing(&self, within: Duration) -> bool {
        self.jukebox_playing_at(Self::now_ms(), within)
    }

    /// `note_jukebox_frame` against a caller-supplied clock reading.
    ///
    /// The clock is a parameter so the window can be exercised across its boundary without a
    /// test sleeping through it.
    pub fn note_jukebox_frame_at(&self, now_ms: u64) {
        self.jukebox_last_frame_ms.store(now_ms, Ordering::Relaxed);
    }

    /// `jukebox_playing` against a caller-supplied clock reading.
    pub fn jukebox_playing_at(&self, now_ms: u64, within: Duration) -> bool {
        let last = self.jukebox_last_frame_ms.load(Ordering::Relaxed);
        if last == 0 {
            return false;
        }

        now_ms.saturating_sub(last) <= within.as_millis() as u64
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or(0)
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
