use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Failed admissions this member's client has served, per joiner and for the whole group.
///
/// Enforced on the admitting client, not the server, so a server guessing the password cannot
/// lift the limit for itself: across N members it gets at most about `5 × N` guesses per window.
pub struct AdmissionRateLimiter {
    per_joiner: HashMap<common::PlayerIdentity, Vec<Instant>>,
    group: Vec<Instant>,
}

impl AdmissionRateLimiter {
    pub const WINDOW: Duration = Duration::from_secs(10 * 60);
    pub const PER_JOINER: usize = 5;
    pub const PER_GROUP: usize = 20;

    pub fn new() -> Self {
        Self {
            per_joiner: HashMap::new(),
            group: Vec::new(),
        }
    }

    pub fn permits(&self, joiner: &common::PlayerIdentity, now: Instant) -> bool {
        let recent = |failures: &Vec<Instant>| {
            failures
                .iter()
                .filter(|at| now.duration_since(**at) < Self::WINDOW)
                .count()
        };
        let joiner_failures = self.per_joiner.get(joiner).map_or(0, recent);
        joiner_failures < Self::PER_JOINER && recent(&self.group) < Self::PER_GROUP
    }

    /// The group-wide limit alone, for a PAKE init whose joiner is not yet known: the identity
    /// arrives with the KeyPackage, after the PAKE.
    pub fn permits_group(&self, now: Instant) -> bool {
        self.group
            .iter()
            .filter(|at| now.duration_since(**at) < Self::WINDOW)
            .count()
            < Self::PER_GROUP
    }

    pub fn record_failure(&mut self, joiner: &common::PlayerIdentity, now: Instant) {
        let fresh = |at: &Instant| now.duration_since(*at) < Self::WINDOW;
        self.group.retain(fresh);
        self.group.push(now);
        let failures = self.per_joiner.entry(joiner.clone()).or_default();
        failures.retain(fresh);
        failures.push(now);
    }
}

impl Default for AdmissionRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
