use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

// A slot held for one probe. Dropping it frees the slot.
pub struct ProbeSlot {
    key: IpAddr,
    held: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl ProbeSlot {
    pub(super) fn new(key: IpAddr, held: Arc<Mutex<HashMap<IpAddr, usize>>>) -> Self {
        Self { key, held }
    }
}

impl Drop for ProbeSlot {
    fn drop(&mut self) {
        let mut held = self.held.lock().expect("the slots are not poisoned");
        if let Some(count) = held.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                held.remove(&self.key);
            }
        }
    }
}
