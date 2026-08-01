use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::LinkQuality;
use crate::structs::reachability::AddressFamily;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LinkDiagnostics {
    pub state: String,
    pub uptime_secs: u64,
    pub rtt_ms: Option<u32>,
    pub rtt_variance_ms: Option<u32>,
    // Packets this client sent that its own loss detection declared lost. Describes the
    // send path only.
    pub uplink_loss_pct: f32,
    // Server-to-client loss, derived locally from gaps in the per-connection sequence the server
    // stamps on every datagram it sends. Nothing skips that sequence, so a single missing value is
    // loss — and because the client computes it from what arrives, the figure stays correct as loss
    // rises rather than depending on a report that travels the same failing path.
    //
    // `None` against a server predating the sequence field, so an unstamped peer reads as unmeasured
    // rather than as a flawless link.
    pub downlink_loss_pct: Option<f32>,
    // Provable loss inferred from QUIC's own packet numbers, kept as a cross-check and as the only
    // downlink signal available against a server with no sequence. A lower bound: runs of two or
    // more consecutive missing numbers only, because the peer skips single numbers deliberately and
    // a receiver cannot tell those from real single losses.
    pub burst_loss_pct: f32,
    // How much of one speaker's audio was fabricated rather than decoded. Not loss, and deliberately
    // not classified as such: a quiet speaker conceals heavily with nothing wrong with the link.
    pub worst_concealment_pct: f32,
    pub jitter_buffer_ms: u32,
    pub jitter_buffer_drops: u64,
    pub quic_port: Option<u16>,
    // Taken from the winning connect candidate, never from an observed socket address: a
    // dual-stack socket dials IPv4 destinations in their v4-mapped form.
    pub family: Option<AddressFamily>,
    pub paths_used: u32,
    pub datagrams_dropped: u64,
    // Sending while nothing returns. A live QUIC connection always produces acknowledgements
    // and keep-alive responses, so this is the client-visible signature of a server that has
    // stopped processing this client's datagrams — path-budget exhaustion being the known
    // cause. Silence in both directions is an idle microphone and is not a stall.
    pub stalled: bool,
    pub quality: LinkQuality,
}
