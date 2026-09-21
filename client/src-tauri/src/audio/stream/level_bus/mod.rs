mod loudness;
mod peer_entry;
mod policy;
mod tracked_peer;

pub use loudness::LoudnessTracker;
pub use peer_entry::PeerEntry;
pub use policy::LevelEmitPolicy;
pub use tracked_peer::TrackedPeer;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use common::structs::audio::{LevelSnapshot, ParticipantLevel};

/// Where every meter's state is collected, and the only thing that publishes it.
///
/// Replaces two independent emitters — one in the capture path, one in the mixer — each of
/// which ran its own 100 ms timer and called `emit` directly. Merging them is not tidiness: on
/// Android each `emit` is a unit of main-thread work, so two streams of them cost twice the
/// main thread for information that is always read together.
///
/// Written from the capture callback, so the self side is atomics only. That thread has a hard
/// deadline and must not allocate or block, and a mutex it shares with a publisher is a lock it
/// can be made to wait on.
pub struct LevelBus {
    own_speaking: AtomicBool,
    own_loudness: AtomicU8,
    // Peers arrive from the mixer's activity task rather than from an audio callback, so a lock
    // is affordable here and a map has to live somewhere.
    peers: Mutex<HashMap<String, PeerEntry>>,
    emitted: AtomicU64,
}

impl LevelBus {
    pub fn new() -> Self {
        Self {
            own_speaking: AtomicBool::new(false),
            own_loudness: AtomicU8::new(0),
            peers: Mutex::new(HashMap::new()),
            emitted: AtomicU64::new(0),
        }
    }

    pub fn new_shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }

    /// Publish this client's own microphone. Called from the capture callback.
    pub fn set_own(&self, level: ParticipantLevel) {
        self.own_speaking.store(level.speaking, Ordering::Relaxed);
        self.own_loudness.store(level.loudness, Ordering::Relaxed);
    }

    /// How long one activity update stands for.
    ///
    /// Six times `ActivityDetector`'s 50 ms emission cooldown, so a peer who is still talking
    /// cannot expire between their own updates, and the same 300 ms the client already treats
    /// as the end of speech.
    pub const PEER_TTL: Duration = Duration::from_millis(300);

    /// Publish one peer's activity.
    pub fn set_peer(&self, name: String, level: ParticipantLevel) {
        self.set_peer_at(name, level, Instant::now());
    }

    /// Publish one peer's activity as of a given moment.
    ///
    /// Separate from `set_peer` so expiry can be exercised without sleeping.
    pub fn set_peer_at(&self, name: String, level: ParticipantLevel, now: Instant) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.insert(name, PeerEntry::new(level, now));
        }
    }

    /// Forget every peer, so a torn-down mixer does not leave meters lit.
    pub fn clear_peers(&self) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.clear();
        }
    }

    /// What would be sent right now.
    pub fn snapshot(&self) -> LevelSnapshot {
        self.snapshot_at(Instant::now())
    }

    /// What would be sent as of a given moment, dropping anyone who has gone quiet.
    ///
    /// Dropped rather than zeroed. A peer absent from a snapshot is what
    /// `LevelEmitPolicy::voices_changed` already reads as having stopped, so the stop reaches
    /// the client immediately; and an entry that is only zeroed still grows the map, the
    /// per-poll scans over it and every serialized payload for the life of the process.
    pub fn snapshot_at(&self, now: Instant) -> LevelSnapshot {
        let peers = match self.peers.lock() {
            Ok(mut peers) => {
                peers.retain(|_, entry| entry.is_fresh(now, Self::PEER_TTL));
                peers
                    .iter()
                    .map(|(name, entry)| (name.clone(), entry.level()))
                    .collect()
            }
            Err(_) => HashMap::new(),
        };

        LevelSnapshot {
            own: ParticipantLevel {
                speaking: self.own_speaking.load(Ordering::Relaxed),
                loudness: self.own_loudness.load(Ordering::Relaxed),
            },
            peers,
        }
    }

    /// Count one published message.
    pub fn record_emitted(&self) {
        self.emitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Messages published since start.
    ///
    /// Monotonic, for the diagnostics service to turn into a rate. This is the number the whole
    /// design exists to hold down, so it is reported rather than assumed: without it, a change
    /// that halves the traffic and a change that does nothing look identical from the outside.
    pub fn emitted(&self) -> u64 {
        self.emitted.load(Ordering::Relaxed)
    }
}

impl Default for LevelBus {
    fn default() -> Self {
        Self::new()
    }
}
