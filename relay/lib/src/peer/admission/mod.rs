pub mod slot;

pub use slot::AdmissionSlot;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

// Bounds what a caller can consume before it is authorized.
//
// The resource model inverted with the transport. A peer used to need a
// certificate the accepting server's own CA had signed before it got a connection
// at all, so the transport was the admission control. Any key can now complete a
// handshake and open a control stream, and only the authority refuses it — after
// the connection exists.
pub struct AdmissionControl {
    in_flight: Arc<AtomicUsize>,
    max_unauthorized: usize,
}

impl AdmissionControl {
    // How long an admitted connection may stay unauthorized. A caller that opens
    // a stream and then says nothing holds a slot and a task without ever
    // identifying itself.
    pub const PRE_AUTH_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new(max_unauthorized: usize) -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            max_unauthorized,
        }
    }

    // `fetch_update` rather than a load-then-store: two callers racing at the cap
    // would both read the same value and both proceed.
    pub fn try_admit(&self) -> Option<AdmissionSlot> {
        let max = self.max_unauthorized;
        self.in_flight
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                (current < max).then_some(current + 1)
            })
            .ok()
            .map(|_| AdmissionSlot::new(Arc::clone(&self.in_flight)))
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }
}
