// Window totals accumulated between rollups, so the event describes an interval rather than an
// instant.
#[derive(Debug, Default, Clone)]
pub struct RollupWindow {
    pub datagrams_sent: u64,
    pub datagrams_received: u64,
    // Numerator and denominator for the window's loss figure, accumulated so it is divided once.
    pub packets_sent: u64,
    pub packets_lost: u64,
    // Envelope-sequence totals for the window, so downlink loss is one division rather than a mean
    // of per-tick ratios.
    pub sequence_received: u64,
    pub sequence_lost: u64,
    pub sequence_measured: bool,
    pub worst_concealment_pct: f32,
    pub underruns: u64,
    pub overflow_drops: u64,
    pub ooo_drops: u64,
    pub plc_frames: u64,
    pub stalled_ticks: u32,
    pub peer_count: u32,
}
