use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::structs::reachability::AddressFamilyPreference;

// A verdict that a synchronous caller can read. The HTTP client builds its
// connector on a non-async path and must not freeze a verdict at construction
// time: a laptop that moves from a v6 network to a v4-only hotspot has to pick up
// the change without being rebuilt.
#[derive(Debug)]
pub struct FamilyPreferenceCell {
    prefer_ipv6: AtomicBool,
}

impl FamilyPreferenceCell {
    pub fn new() -> Self {
        Self {
            prefer_ipv6: AtomicBool::new(false),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn get(&self) -> AddressFamilyPreference {
        if self.prefer_ipv6.load(Ordering::Relaxed) {
            AddressFamilyPreference::PreferIpv6
        } else {
            AddressFamilyPreference::PreferIpv4
        }
    }

    pub fn set(&self, preference: AddressFamilyPreference) {
        let prefer_ipv6 = matches!(preference, AddressFamilyPreference::PreferIpv6);
        self.prefer_ipv6.store(prefer_ipv6, Ordering::Relaxed);
    }
}

impl Default for FamilyPreferenceCell {
    fn default() -> Self {
        Self::new()
    }
}
