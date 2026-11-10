use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use moka::sync::Cache;

// How often a node, and an address, may ask the registry to dial them.
//
// A preflight makes the registry connect outward, so unmetered it is a way to aim
// connections at a third party. The per-address allowance stops one machine from
// multiplying its allowance by generating node keys.
pub struct PlayPreflightLimiter {
    node_window: Duration,
    per_address_per_hour: u32,
    nodes: Cache<String, Instant>,
    // Window start and count, per address.
    addresses: Cache<IpAddr, (Instant, u32)>,
    // Held across the reads and writes of one admission, so concurrent requests cannot
    // all pass a check that only the first of them would.
    admission: Mutex<()>,
}

impl PlayPreflightLimiter {
    pub const NODE_WINDOW: Duration = Duration::from_secs(15);
    pub const PER_ADDRESS_PER_HOUR: u32 = 30;
    pub const OBSERVED_V6_PREFIX_BITS: u32 = 56;
    const CANDIDATE_V6_PREFIX_BITS: u32 = 64;
    const ADDRESS_WINDOW: Duration = Duration::from_secs(3600);
    const CAPACITY: u64 = 100_000;

    pub fn new(node_window: Duration, per_address_per_hour: u32) -> Self {
        Self {
            node_window,
            per_address_per_hour,
            nodes: Cache::builder()
                .time_to_live(node_window)
                .max_capacity(Self::CAPACITY)
                .build(),
            addresses: Cache::builder()
                .time_to_live(Self::ADDRESS_WINDOW)
                .max_capacity(Self::CAPACITY)
                .build(),
            admission: Mutex::new(()),
        }
    }

    // `candidate` is the address the registry would dial. `observed` is the address the
    // request arrived from. Both are metered, and the request is refused when either is
    // over its allowance. Metering only the candidate would let one machine aim the
    // registry at any number of targets by changing the candidate.
    //
    // `Err` carries how long until a retry is admitted.
    pub fn admit(
        &self,
        node_id: &str,
        candidate: IpAddr,
        observed: Option<IpAddr>,
    ) -> Result<(), Duration> {
        let _guard = self.admission.lock().expect("the limiter is not poisoned");
        let now = Instant::now();

        if let Some(at) = self.nodes.get(node_id) {
            let wait = self
                .node_window
                .saturating_sub(now.saturating_duration_since(at));
            if !wait.is_zero() {
                return Err(wait);
            }
        }

        let mut keys = vec![Self::address_key(candidate)];
        if let Some(observed) = observed {
            let key = Self::observed_key(observed);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }

        let mut windows = Vec::with_capacity(keys.len());
        for key in keys {
            let (started, count) = match self.addresses.get(&key) {
                Some((started, count))
                    if now.saturating_duration_since(started) < Self::ADDRESS_WINDOW =>
                {
                    (started, count)
                }
                _ => (now, 0),
            };
            if count >= self.per_address_per_hour {
                return Err(
                    Self::ADDRESS_WINDOW.saturating_sub(now.saturating_duration_since(started))
                );
            }
            windows.push((key, started, count));
        }

        for (key, started, count) in windows {
            self.addresses.insert(key, (started, count + 1));
        }
        self.nodes.insert(node_id.to_string(), now);
        Ok(())
    }

    // The key a requester's own address is metered under: IPv4 per address, IPv6 per /56,
    // because a subscriber can hold a /56 or a /48 and so many /64 allowances.
    pub fn observed_key(address: IpAddr) -> IpAddr {
        Self::masked(address, Self::OBSERVED_V6_PREFIX_BITS)
    }

    // The key a dial target is metered under: IPv4 per address, IPv6 per /64.
    fn address_key(address: IpAddr) -> IpAddr {
        Self::masked(address, Self::CANDIDATE_V6_PREFIX_BITS)
    }

    fn masked(address: IpAddr, v6_prefix_bits: u32) -> IpAddr {
        match address.to_canonical() {
            IpAddr::V4(v4) => IpAddr::V4(v4),
            IpAddr::V6(v6) => {
                let masked = u128::from(v6) & (u128::MAX << (128 - v6_prefix_bits));
                IpAddr::V6(masked.into())
            }
        }
    }
}
