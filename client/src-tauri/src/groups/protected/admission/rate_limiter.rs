use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Checks that failed, per joiner identity and for the whole group, on this member's client.
///
/// Each check a server attacks gives it one chance in 10⁸ to match both codes. Enforced here, not
/// on the server, so a server cannot lift the limit for itself. A failed check and a check that
/// expired with no reveal count; `Busy`, and a check the member let expire after a valid reveal,
/// do not.
pub struct AdmissionRateLimiter {
    per_joiner: HashMap<String, Vec<Instant>>,
    group: Vec<Instant>,
}

impl AdmissionRateLimiter {
    pub const JOINER_WINDOW: Duration = Duration::from_secs(10 * 60);
    pub const GROUP_WINDOW: Duration = Duration::from_secs(60 * 60);
    pub const PER_JOINER: usize = 3;
    pub const PER_GROUP: usize = 10;

    pub fn new() -> Self {
        Self {
            per_joiner: HashMap::new(),
            group: Vec::new(),
        }
    }

    /// Whether this member may start another check of `identity`.
    pub fn permits_check(&self, identity: &str, now: Instant) -> bool {
        let joiner_failures = self.per_joiner.get(identity).map_or(0, |failures| {
            failures
                .iter()
                .filter(|at| now.duration_since(**at) < Self::JOINER_WINDOW)
                .count()
        });
        let group_failures = self
            .group
            .iter()
            .filter(|at| now.duration_since(**at) < Self::GROUP_WINDOW)
            .count();
        joiner_failures < Self::PER_JOINER && group_failures < Self::PER_GROUP
    }

    pub fn record_failure(&mut self, identity: &str, now: Instant) {
        self.group
            .retain(|at| now.duration_since(*at) < Self::GROUP_WINDOW);
        self.group.push(now);
        let failures = self.per_joiner.entry(identity.to_string()).or_default();
        failures.retain(|at| now.duration_since(*at) < Self::JOINER_WINDOW);
        failures.push(now);
    }
}

impl Default for AdmissionRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
