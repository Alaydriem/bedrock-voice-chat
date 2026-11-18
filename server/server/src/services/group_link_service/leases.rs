use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;

/// Channels the Simple Voice Chat bridge is keeping linked.
///
/// A linked group can hold only Java players, who never hold a BVC client, so the reaper would
/// close it while it is still in use. The bridge renews its channels on a timer; a bridge that
/// stops renewing has gone, and its leases lapse.
pub struct GroupLinkLeases {
    leased: Cache<String, ()>,
}

impl GroupLinkLeases {
    /// Three missed renewals at the bridge's 10 second interval.
    pub const TTL: Duration = Duration::from_secs(30);
    const CAPACITY: u64 = 4096;

    /// `ttl` is `Self::TTL` everywhere but a test that has to see a lease lapse.
    pub fn new(ttl: Duration) -> Self {
        Self {
            leased: Cache::builder()
                .time_to_live(ttl)
                .max_capacity(Self::CAPACITY)
                .build(),
        }
    }

    pub fn new_shared(ttl: Duration) -> Arc<Self> {
        Arc::new(Self::new(ttl))
    }

    /// The caller filters out protected channels: they are never linked.
    pub fn renew(&self, channels: &[String]) {
        for channel in channels {
            self.leased.insert(channel.clone(), ());
        }
    }

    pub fn is_leased(&self, channel_id: &str) -> bool {
        self.leased.contains_key(channel_id)
    }
}
