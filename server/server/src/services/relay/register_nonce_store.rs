use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;

// Short TTL covering the challenge -> verify -> register round trip. Long enough
// for the relay's reachability callback, short enough that a stray nonce is not
// servable indefinitely.
const NONCE_TTL: Duration = Duration::from_secs(120);

const NONCE_CAPACITY: u64 = 256;

// Registrant-side store of nonces this server was issued by the relay during a
// registration challenge. When the relay performs its endpoint-control
// reachability callback (`GET /relay/proof/<nonce>`), this
// server's proof route answers only for nonces it actually holds here — proving
// it received the challenge for the endpoint it is claiming.
//
// An attacker pointing the relay at a victim's endpoint cannot succeed: the
// victim never received the attacker's nonce, so the victim's proof route does
// not serve it.
#[derive(Clone)]
pub struct RegisterNonceStore {
    cache: Arc<Cache<String, ()>>,
}

impl RegisterNonceStore {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(NONCE_TTL)
            .max_capacity(NONCE_CAPACITY)
            .build();
        Self {
            cache: Arc::new(cache),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // Records a nonce the relay issued to us so our proof route can serve it.
    pub fn remember(&self, nonce: &str) {
        self.cache.insert(nonce.to_string(), ());
    }

    // True iff we were issued `nonce` (and it has not expired).
    pub fn contains(&self, nonce: &str) -> bool {
        self.cache.get(&nonce.to_string()).is_some()
    }
}

impl Default for RegisterNonceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_nonce_is_served_others_are_not() {
        let store = RegisterNonceStore::new();
        store.remember("good-nonce");
        assert!(store.contains("good-nonce"));
        assert!(!store.contains("attacker-nonce"));
    }
}
