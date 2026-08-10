use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};
use std::num::NonZeroU32;
use std::sync::Arc;

type KeyedLimiter = RateLimiter<
    String,
    DefaultKeyedStateStore<String>,
    DefaultClock,
    NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
>;

/// Quotas for the two unauthenticated relay code endpoints.
///
/// Keyed on what each request is *about* rather than on where it came from. The public
/// port is fronted by the TLS demultiplexer, which relays from loopback, so every caller
/// shares one address and an address-keyed limiter would hold a single global bucket —
/// one busy peer would then throttle the whole mesh. The same is already true of any
/// deployment behind a proxy.
///
/// The binding fields are the better key regardless. An offer is bounded per world,
/// which is the realm the inject lands in; a redemption is bounded per presenter, which
/// is the recipient a code is sealed to. An attacker can vary either, but varying it
/// moves them off the target they were attacking — where varying a source address does
/// not.
pub struct RelayRateLimiter {
    offers: KeyedLimiter,
    redemptions: KeyedLimiter,
}

impl RelayRateLimiter {
    // Sized ABOVE the relay's own legitimate cadence, not just against brute force: the
    // orchestrator re-offers every 15s while a link is unproven (~4 offer+redeem pairs
    // per minute per dialing peer). Codes are single-use, short-TTL and high-entropy, so
    // this is a resource bound rather than the security boundary.
    pub const PER_MINUTE: u32 = 30;

    // Keyed state grows one entry per distinct key and governor never prunes on its own.
    // Sweeping once the map passes this bound keeps it proportional to live traffic
    // instead of to everything ever seen.
    const SWEEP_THRESHOLD: usize = 4096;

    pub fn new() -> Self {
        Self {
            offers: RateLimiter::keyed(Self::quota()),
            redemptions: RateLimiter::keyed(Self::quota()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Whether an offer for this world may proceed.
    pub fn allow_offer(&self, hashed_world: &str) -> bool {
        Self::check(&self.offers, hashed_world)
    }

    /// Whether a redemption naming this presenter may proceed.
    pub fn allow_redemption(&self, presenter: &str) -> bool {
        Self::check(&self.redemptions, presenter)
    }

    fn quota() -> Quota {
        Quota::per_minute(
            NonZeroU32::new(Self::PER_MINUTE).expect("relay quota is a non-zero constant"),
        )
    }

    fn check(limiter: &KeyedLimiter, key: &str) -> bool {
        if limiter.len() > Self::SWEEP_THRESHOLD {
            limiter.retain_recent();
        }

        limiter.check_key(&key.to_string()).is_ok()
    }
}

impl Default for RelayRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
