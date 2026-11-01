use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

// This server's minted peer link, served without waiting on the registry.
//
// A stale link is still served while its replacement is minted. The address it carries
// rarely changes, and a reader holding a link a few minutes old loses nothing, whereas a
// reader made to wait for a fresh one waits on a registry round trip.
//
// At most one refresh runs at a time. Readers that find the link stale while a refresh is
// already running take the stale one rather than starting their own.
pub struct PeerTicketCache {
    ttl: Duration,
    current: RwLock<Option<(String, Instant)>>,
    refreshing: AtomicBool,
}

impl PeerTicketCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            current: RwLock::new(None),
            refreshing: AtomicBool::new(false),
        }
    }

    /// The last link minted, however old.
    pub fn get(&self) -> Option<String> {
        self.current
            .read()
            .ok()
            .and_then(|current| current.as_ref().map(|(ticket, _)| ticket.clone()))
    }

    /// Whether the caller should mint a replacement. True for exactly one caller while the
    /// link is missing or stale; that caller must end with `store` or `release`.
    pub fn claim_refresh(&self) -> bool {
        if !self.needs_refresh() {
            return false;
        }

        self.refreshing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Records a freshly minted link and ends the refresh.
    pub fn store(&self, ticket: String) {
        if let Ok(mut current) = self.current.write() {
            *current = Some((ticket, Instant::now()));
        }
        self.refreshing.store(false, Ordering::SeqCst);
    }

    /// Ends a refresh that produced nothing, so the next reader tries again.
    pub fn release(&self) {
        self.refreshing.store(false, Ordering::SeqCst);
    }

    fn needs_refresh(&self) -> bool {
        match self.current.read() {
            Ok(current) => match current.as_ref() {
                Some((_, minted)) => minted.elapsed() >= self.ttl,
                None => true,
            },
            Err(_) => true,
        }
    }
}
