// Transport-fidelity counters for the e2e harness, read at the true network boundaries that
// prove end-to-end delivery:
//
//   frames_sent         — AudioFrame datagrams this client actually handed to QUIC (counted
//                         on send success, at the real send_datagram call).
//   frames_from_quic    — AudioFrame datagrams taken off the QUIC bus (post-network,
//                         pre-decode-routing). These originate from other players routed by
//                         the server.
//   frames_into_jitter_buffer
//                       — EncodedAudioFramePackets forwarded into the jitter buffer pipeline.
//                         This is BEFORE playback drain, so it is not a "heard" count.
//
// The meaningful lossless assertion is end-to-end across two processes:
//   sender frames_sent == receiver frames_from_quic
// i.e. every AudioFrame datagram one client handed to QUIC arrived at the peer from QUIC. The
// fake input source paces frames at a real 20 ms cadence so the bounded datagram queue never
// overruns, which is what makes this exact equality hold.
//
// The "heard" assertion (notes present, rms above floor) is separate and depends on actual
// Opus decode, not just frame accounting.
//
// The storage lives in `TransportStats` rather than here, because the same counters now feed
// the live diagnostics snapshot: a second set of statics would let the harness and the shipped
// numbers disagree about the same event. `TransportStats` is injected into the network streams
// for testability, and this module reads whichever instance the running app registered.

use std::sync::Arc;
use std::sync::OnceLock;

use crate::diagnostics::TransportStats;

static REGISTERED: OnceLock<Arc<TransportStats>> = OnceLock::new();

pub struct TransportCounters;

impl TransportCounters {
    // Called once during app setup with the same instance the network streams received.
    pub fn register(stats: Arc<TransportStats>) {
        let _ = REGISTERED.set(stats);
    }

    pub fn increment_into_jitter_buffer() {
        if let Some(stats) = REGISTERED.get() {
            stats.record_frame_into_jitter_buffer();
        }
    }

    // Zeros when nothing was registered, which happens only in unit tests that never boot an
    // app. A harness run always registers, so a zero there is a real observation rather than
    // an unwired counter.
    pub fn snapshot() -> (u64, u64, u64) {
        match REGISTERED.get() {
            Some(stats) => (
                stats.frames_sent(),
                stats.frames_from_quic(),
                stats.frames_into_jitter_buffer(),
            ),
            None => (0, 0, 0),
        }
    }
}
