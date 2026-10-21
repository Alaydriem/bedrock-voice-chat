use std::collections::HashMap;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// KeyPackages this group's member checked recently, by hash. A KeyPackage is used once; one
/// seen again within the window is a replay.
#[derive(Default)]
pub struct SeenKeyPackages {
    seen: HashMap<[u8; 32], Instant>,
}

impl SeenKeyPackages {
    pub const WINDOW: Duration = Duration::from_secs(10 * 60);

    pub fn new() -> Self {
        Self::default()
    }

    /// Records the KeyPackage. Returns `false` when it was already seen within the window.
    pub fn record(&mut self, key_package: &[u8], now: Instant) -> bool {
        self.seen
            .retain(|_, at| now.duration_since(*at) < Self::WINDOW);
        let hash: [u8; 32] = Sha256::digest(key_package).into();
        if self.seen.contains_key(&hash) {
            return false;
        }
        self.seen.insert(hash, now);
        true
    }
}
