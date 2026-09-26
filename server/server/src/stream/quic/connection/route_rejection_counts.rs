use std::sync::atomic::{AtomicU64, Ordering};

use crate::services::metrics_service::interaction::RouteRejection;

/// Per-reason rejection counters the registry owns.
///
/// Exists beside the Prometheus counter because a test cannot read a Prometheus recorder, and
/// because the registry has to be able to answer "what did you refuse, and why" without one
/// installed. Relaxed: observational, on the hot path, never used to order other work.
#[derive(Debug)]
pub struct RouteRejectionCounts {
    counts: [AtomicU64; RouteRejection::ALL.len()],
}

impl RouteRejectionCounts {
    pub fn new() -> Self {
        Self {
            counts: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    pub fn record(&self, reason: RouteRejection) {
        self.counts[reason.index()].fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, reason: RouteRejection) -> u64 {
        self.counts[reason.index()].load(Ordering::Relaxed)
    }
}

impl Default for RouteRejectionCounts {
    fn default() -> Self {
        Self::new()
    }
}
