use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use common::structs::relay::wire::datagram::VoiceFrame;
use tokio::sync::Notify;

// Inbound voice waiting to be read, bounded and drop-oldest.
//
// A queue that grows delivers stale audio late, which is worse than delivering
// none: the consumer has no way to tell that what it is playing is behind. The
// bound is a handful of frames, so a consumer that stalls loses the stall rather
// than accumulating it.
//
// This is not a jitter buffer. It reorders nothing, waits for nothing, and holds
// no playout deadline — the consumer has its own buffering, and two in series is
// two sets of latency.
pub struct Inbox {
    frames: Mutex<VecDeque<VoiceFrame>>,
    capacity: usize,
    closed: AtomicBool,
    ready: Notify,
}

impl Inbox {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);

        Self {
            frames: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            closed: AtomicBool::new(false),
            ready: Notify::new(),
        }
    }

    pub fn push(&self, frame: VoiceFrame) {
        if self.closed.load(Ordering::SeqCst) {
            return;
        }

        {
            let mut frames = self.frames.lock().expect("inbox lock");
            if frames.len() == self.capacity {
                frames.pop_front();
            }
            frames.push_back(frame);
        }

        self.ready.notify_one();
    }

    // `None` once the inbox is closed and drained.
    //
    // The queue is checked before the closed flag so a close does not discard
    // frames that already arrived: the caller asked to stop reading, not to throw
    // away what it had.
    pub async fn next(&self) -> Option<VoiceFrame> {
        loop {
            // Registered before the queue is inspected. Registering after would
            // lose a push or a close landing in between, parking a reader that
            // nothing will wake again.
            let notified = self.ready.notified();

            if let Some(frame) = self.frames.lock().expect("inbox lock").pop_front() {
                return Some(frame);
            }

            if self.closed.load(Ordering::SeqCst) {
                return None;
            }

            notified.await;
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.ready.notify_waiters();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}
