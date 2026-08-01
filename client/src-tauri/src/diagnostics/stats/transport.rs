use std::sync::atomic::{AtomicU64, Ordering};

// Datagram accounting at the two real network boundaries: what this client handed to QUIC,
// and what it took back off the wire.
//
// The three frame counters exist so the e2e harness's transport-fidelity assertions have a
// single source. They were previously statics compiled only under the `e2e` feature; the
// harness API is unchanged, but the storage now lives here so production builds can report
// rates from the same numbers the tests trust.
#[derive(Debug, Default)]
pub struct TransportStats {
    datagrams_sent: AtomicU64,
    datagrams_received: AtomicU64,
    send_errors: AtomicU64,
    frames_sent: AtomicU64,
    frames_from_quic: AtomicU64,
    frames_into_jitter_buffer: AtomicU64,
}

impl TransportStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sent(&self) {
        self.datagrams_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_received(&self) {
        self.datagrams_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_send_error(&self) {
        self.send_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_frame_sent(&self) {
        self.frames_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_frame_from_quic(&self) {
        self.frames_from_quic.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_frame_into_jitter_buffer(&self) {
        self.frames_into_jitter_buffer
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn datagrams_sent(&self) -> u64 {
        self.datagrams_sent.load(Ordering::Relaxed)
    }

    pub fn datagrams_received(&self) -> u64 {
        self.datagrams_received.load(Ordering::Relaxed)
    }

    pub fn send_errors(&self) -> u64 {
        self.send_errors.load(Ordering::Relaxed)
    }

    pub fn frames_sent(&self) -> u64 {
        self.frames_sent.load(Ordering::Relaxed)
    }

    pub fn frames_from_quic(&self) -> u64 {
        self.frames_from_quic.load(Ordering::Relaxed)
    }

    pub fn frames_into_jitter_buffer(&self) -> u64 {
        self.frames_into_jitter_buffer.load(Ordering::Relaxed)
    }
}
