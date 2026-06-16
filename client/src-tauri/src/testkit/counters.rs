// Transport-fidelity counters for the e2e harness. Incremented on the real
// audio path at the true network boundaries that prove end-to-end delivery:
//
//   frames_sent         — AudioFrame datagrams this client actually handed to
//                         QUIC (the real send_datagram call in the network
//                         output path, counted on send success).
//   frames_from_quic    — AudioFrame datagrams this client received from the
//                         QUIC bus (post-network, pre-decode-routing). These
//                         originate from other players routed by the server.
//   frames_into_jitter_buffer
//                       — EncodedAudioFramePackets forwarded into the jitter
//                         buffer pipeline (after handle_audio_data succeeds).
//                         This is BEFORE playback/decode drain, so it is not a
//                         "heard" count — it measures ingest into the pipeline.
//
// The meaningful lossless assertion is end-to-end across two processes:
//   sender frames_sent == receiver frames_from_quic
// i.e. every AudioFrame datagram one client handed to QUIC arrived at the
// peer from QUIC — zero loss over the loopback transport. The fake input
// source paces frames at a real 20 ms cadence so the bounded datagram queue
// never overruns, which is what makes this exact equality hold.
//
// The "heard" assertion (notes present, rms above floor) is separate and
// depends on actual Opus decode, not just frame accounting.
//
// All counters are process-global AtomicU64s compiled only under the `e2e`
// feature so production builds carry no overhead.

use std::sync::atomic::{AtomicU64, Ordering};

pub static FRAMES_SENT: AtomicU64 = AtomicU64::new(0);
pub static FRAMES_FROM_QUIC: AtomicU64 = AtomicU64::new(0);
pub static FRAMES_INTO_JITTER_BUFFER: AtomicU64 = AtomicU64::new(0);

pub struct TransportCounters;

impl TransportCounters {
    pub fn increment_sent() {
        FRAMES_SENT.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_from_quic() {
        FRAMES_FROM_QUIC.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_into_jitter_buffer() {
        FRAMES_INTO_JITTER_BUFFER.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot() -> (u64, u64, u64) {
        (
            FRAMES_SENT.load(Ordering::Relaxed),
            FRAMES_FROM_QUIC.load(Ordering::Relaxed),
            FRAMES_INTO_JITTER_BUFFER.load(Ordering::Relaxed),
        )
    }
}
