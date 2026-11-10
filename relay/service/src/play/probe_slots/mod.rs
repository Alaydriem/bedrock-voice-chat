use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

mod slot;

pub use slot::ProbeSlot;

// Counts the probes in flight per requester, so one requester cannot hold every permit of
// the global cap.
pub struct ProbeSlots {
    per_key: usize,
    held: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl ProbeSlots {
    pub fn new(per_key: usize) -> Self {
        Self {
            per_key,
            held: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // `None` when `key` already holds its share.
    pub fn acquire(&self, key: IpAddr) -> Option<ProbeSlot> {
        let mut held = self.held.lock().expect("the slots are not poisoned");
        let count = held.entry(key).or_insert(0);
        if *count >= self.per_key {
            return None;
        }
        *count += 1;

        Some(ProbeSlot::new(key, self.held.clone()))
    }
}
