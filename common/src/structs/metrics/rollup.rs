use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::reachability::{AddressFamily, AddressFamilyPreference};

// A windowed summary of this client's link to its server, shaped for off-device analytics.
//
// Privacy is structural, not a convention: there is no field for a player name, a peer
// identity, a peer hash, or a location. The client's own region comes from the ingest IP at
// the analytics provider, and the server is identified only by a hash of its CA.
//
// Percentiles rather than averages, because an average round trip stops meaning anything
// once it is aggregated across users while a 95th percentile still does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LinkRollup {
    pub server_id: String,
    pub rtt_p50_ms: Option<u32>,
    pub rtt_p95_ms: Option<u32>,
    pub rtt_max_ms: Option<u32>,
    // Accumulated over the whole window and divided once, not an average of per-tick ratios: a tick
    // with one packet sent and one lost is 100% and would otherwise weigh as much as a tick with
    // fifty sent.
    pub uplink_loss_pct: f32,
    // Server-to-client loss from the server's own per-connection sequence, accumulated over the
    // window and divided once. `None` against a server predating the sequence field, so an
    // unmeasured window is distinguishable from a clean one — an important difference when the
    // question being asked is whether a region has loss at all.
    pub downlink_loss_pct: Option<f32>,
    pub worst_concealment_pct: f32,
    pub datagrams_sent: u64,
    pub datagrams_received: u64,
    pub underruns: u64,
    pub overflow_drops: u64,
    pub ooo_drops: u64,
    pub plc_frames: u64,
    pub peer_count: u32,
    // Distinguishes a short window from a quiet one.
    pub samples: u32,
    // Ticks in the window where this client sent and nothing came back.
    pub stalled_ticks: u32,
    pub address_family: Option<AddressFamily>,
    pub family_preference: Option<AddressFamilyPreference>,
    pub protocol_version: String,
    pub client_version: String,
}
