use std::sync::{Arc, Mutex};

// The challenge value the relay most recently sent over the enrollment session.
//
// Published over an unauthenticated route so the relay can fetch it from the address
// the operator declared. That fetch is what binds the address record to this node:
// without it, an operator could declare an address they do not control and pass the
// identity half forever, because their node is genuinely fine.
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
