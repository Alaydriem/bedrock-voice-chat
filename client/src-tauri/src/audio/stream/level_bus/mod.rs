mod loudness;
mod policy;

pub use loudness::LoudnessTracker;
pub use policy::LevelEmitPolicy;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

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
    peers: Mutex<HashMap<String, ParticipantLevel>>,
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

    /// Publish one peer's activity.
    pub fn set_peer(&self, name: String, level: ParticipantLevel) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.insert(name, level);
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
        LevelSnapshot {
            own: ParticipantLevel {
                speaking: self.own_speaking.load(Ordering::Relaxed),
                loudness: self.own_loudness.load(Ordering::Relaxed),
            },
            peers: self
                .peers
                .lock()
                .map(|peers| peers.clone())
                .unwrap_or_default(),
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
