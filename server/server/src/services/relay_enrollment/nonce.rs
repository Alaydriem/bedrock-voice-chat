use std::sync::{Arc, Mutex};

// The nonce from the most recent check-in, served on an unauthenticated route.
pub struct CurrentNonce {
    value: Mutex<Option<String>>,
}

impl CurrentNonce {
    pub fn new() -> Self {
        Self {
            value: Mutex::new(None),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set(&self, value: String) {
        *self.value.lock().expect("nonce lock") = Some(value);
    }

    pub fn get(&self) -> Option<String> {
        self.value.lock().expect("nonce lock").clone()
    }
}

impl Default for CurrentNonce {
    fn default() -> Self {
        Self::new()
    }
}
