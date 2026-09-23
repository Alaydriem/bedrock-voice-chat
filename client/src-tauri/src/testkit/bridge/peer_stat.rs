use common::structs::metrics::PeerDiagnostics;
use serde::{Deserialize, Serialize};

/// The per-speaker counters a scenario asserts on. A subset of `PeerDiagnostics`: the full
/// record is not worth serialising through the bridge, and a scenario that needs a new field
/// adds it here deliberately.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerStat {
    pub name: String,
    pub ooo_drops: u64,
    pub frames_decoded: u64,
    pub plc_frames: u64,
    pub underruns: u64,
    pub reanchors: u64,
    pub warmup_rearms: u64,
    pub drain_sheds: u64,
    pub spatial_gap_frames: u64,
    pub normal_gap_frames: u64,
}

impl PeerStat {
    pub fn from_diagnostics(peer: &PeerDiagnostics) -> Self {
        Self {
            name: peer.name.clone(),
            ooo_drops: peer.ooo_drops,
            frames_decoded: peer.frames_decoded,
            plc_frames: peer.plc_frames,
            underruns: peer.underruns,
            reanchors: peer.reanchors,
            warmup_rearms: peer.warmup_rearms,
            drain_sheds: peer.drain_sheds,
            spatial_gap_frames: peer.spatial_gap_frames,
            normal_gap_frames: peer.normal_gap_frames,
        }
    }
}
