use common::structs::metrics::{LinkDiagnosticsSnapshot, LinkRollup};
use common::structs::reachability::{AddressFamily, AddressFamilyPreference};

use super::RollupWindow;
use crate::diagnostics::SampleRing;


// Builds the off-device summary of this client's link to its server.
//
// There is deliberately no peer dimension. The path is client to server to peer, so a listener
// and a speaker each measure their own leg and nothing here needs to name the other party — which
// is what keeps a stable cross-machine pseudonym, and the co-presence graph it would imply, off
// the wire entirely.
pub struct RollupBuilder;

impl RollupBuilder {
    pub fn build(
        server_id: String,
        snapshot: &LinkDiagnosticsSnapshot,
        ring: &SampleRing,
        window: &RollupWindow,
        family: Option<AddressFamily>,
        family_preference: Option<AddressFamilyPreference>,
        client_version: String,
    ) -> LinkRollup {
        LinkRollup {
            server_id,
            rtt_p50_ms: ring.rtt_percentile(50.0),
            rtt_p95_ms: ring.rtt_percentile(95.0),
            rtt_max_ms: ring.rtt_max(),
            uplink_loss_pct: Self::loss_pct(window.packets_lost, window.packets_sent),
            downlink_loss_pct: window.sequence_measured.then(|| {
                Self::loss_pct(
                    window.sequence_lost,
                    window.sequence_lost + window.sequence_received,
                )
            }),
            worst_concealment_pct: window.worst_concealment_pct,
            datagrams_sent: window.datagrams_sent,
            datagrams_received: window.datagrams_received,
            underruns: window.underruns,
            overflow_drops: window.overflow_drops,
            ooo_drops: window.ooo_drops,
            plc_frames: window.plc_frames,
            peer_count: window.peer_count,
            samples: ring.len() as u32,
            stalled_ticks: window.stalled_ticks,
            address_family: family,
            family_preference,
            protocol_version: snapshot
                .session
                .protocol_version
                .clone()
                .unwrap_or_default(),
            client_version,
        }
    }

    // One division over the accumulated window. Averaging per-tick ratios would let a tick with a
    // single packet sent and lost count 100% and weigh as much as a tick with fifty sent.
    fn loss_pct(lost: u64, sent: u64) -> f32 {
        if sent == 0 {
            return 0.0;
        }
        ((lost as f64 / sent as f64) * 100.0).clamp(0.0, 100.0) as f32
    }

    // An idle client is not a data point about link quality, so a window with nothing in it
    // produces no event rather than a row of zeros that would drag every average toward perfect.
    pub fn is_reportable(ring: &SampleRing, window: &RollupWindow) -> bool {
        !ring.is_empty() && (window.datagrams_sent > 0 || window.datagrams_received > 0)
    }
}
