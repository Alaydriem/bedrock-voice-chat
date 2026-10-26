use moka::sync::Cache;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Which gamerpic requests to one server are worth making.
///
/// The dashboard asks again every few seconds for each nearby player without a picture. Without
/// this, a server that refuses this device's certificate, or a player with no picture, costs one
/// request and one log line per player per sync, forever.
#[derive(Debug)]
pub struct GamerpicGate {
    refused: AtomicBool,
    misses: Cache<String, ()>,
}

impl GamerpicGate {
    const MISS_TTL: Duration = Duration::from_secs(600);
    const MISS_CAPACITY: u64 = 512;

    pub fn new() -> Self {
        Self {
            refused: AtomicBool::new(false),
            misses: Cache::builder()
                .time_to_live(Self::MISS_TTL)
                .max_capacity(Self::MISS_CAPACITY)
                .build(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Close the gate for the rest of this session. True only for the call that closed it, so
    /// the follow-up probe runs once however many requests were refused together.
    pub fn refuse(&self) -> bool {
        !self.refused.swap(true, Ordering::SeqCst)
    }

    /// Whether a refusal with this status is worth asking `/api/auth/state` about.
    ///
    /// A 401 is `PlayerGuard`'s missing certificate: nothing reached the server, which is what a
    /// TLS-terminating proxy produces while voice on the same certificate works. A certificate the
    /// server distrusts fails the handshake instead, so the probe would answer 401 for the same
    /// proxy and sign out a player whose credentials are fine. Only a 403 means the server saw
    /// the certificate and refused it.
    pub fn refusal_needs_probe(status: u16) -> bool {
        status == 403
    }

    pub fn is_refused(&self) -> bool {
        self.refused.load(Ordering::SeqCst)
    }

    pub fn record_miss(&self, game: &str, gamertag: &str) {
        self.misses.insert(Self::key(game, gamertag), ());
    }

    pub fn is_miss(&self, game: &str, gamertag: &str) -> bool {
        self.misses.contains_key(&Self::key(game, gamertag))
    }

    fn key(game: &str, gamertag: &str) -> String {
        format!("{game}:{gamertag}")
    }
}

impl Default for GamerpicGate {
    fn default() -> Self {
        Self::new()
    }
}
