use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use common::structs::metrics::PeerDiagnostics;

// Each audio frame advances the timestamp by one Opus frame duration, which is what makes a
// gap in the sequence measurable as loss.
const FRAME_DURATION_MS: u64 = 20;

// Per-speaker receive counters, published outward from the playback thread.
//
// This type exists because `JitterBufferSource` is moved into rodio's graph and no handle to
// it survives construction — the counters cannot be read from outside, so they have to be
// written outward into shared state created before the move.
#[derive(Debug, Default)]
pub struct PlayerReceiveStats {
    name: String,
    underruns: AtomicU64,
    overflow_drops: AtomicU64,
    ooo_drops: AtomicU64,
    plc_frames: AtomicU64,
    silence_frames: AtomicU64,
    frames_decoded: AtomicU64,
    frames_received: AtomicU64,
    ring_len: AtomicU32,
    capacity: AtomicU32,
    warmup_needed: AtomicU32,
    // Distinguishes "no packet has arrived" from "a packet arrived bearing timestamp 0".
    seen_any: AtomicU32,
}

impl PlayerReceiveStats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // The ring was drained and playback needed a frame that had not arrived.
    pub fn record_underrun(&self) {
        self.underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_overflow_drop(&self) {
        self.overflow_drops.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ooo_drop(&self) {
        self.ooo_drops.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_plc(&self) {
        self.plc_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_silence(&self) {
        self.silence_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_decode(&self, frames: usize) {
        self.frames_decoded
            .fetch_add(frames as u64, Ordering::Relaxed);
    }

    // Frames advance a wall-clock capture stamp, not a sequence number, so an arrival carries no
    // information about whether anything was missed before it — only that something came. The
    // timestamp is deliberately unused here; see the note on `LinkDiagnostics`.
    pub fn record_arrival(&self, _timestamp_ms: u64) {
        self.frames_received.fetch_add(1, Ordering::Relaxed);
        self.seen_any.store(1, Ordering::Relaxed);
    }

    pub fn set_ring(&self, ring_len: usize, capacity: usize, warmup_needed: usize) {
        self.ring_len.store(ring_len as u32, Ordering::Relaxed);
        self.capacity.store(capacity as u32, Ordering::Relaxed);
        self.warmup_needed
            .store(warmup_needed as u32, Ordering::Relaxed);
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    pub fn overflow_drops(&self) -> u64 {
        self.overflow_drops.load(Ordering::Relaxed)
    }

    pub fn ooo_drops(&self) -> u64 {
        self.ooo_drops.load(Ordering::Relaxed)
    }

    pub fn plc_frames(&self) -> u64 {
        self.plc_frames.load(Ordering::Relaxed)
    }

    pub fn silence_frames(&self) -> u64 {
        self.silence_frames.load(Ordering::Relaxed)
    }

    pub fn frames_decoded(&self) -> u64 {
        self.frames_decoded.load(Ordering::Relaxed)
    }

    pub fn frames_received(&self) -> u64 {
        self.frames_received.load(Ordering::Relaxed)
    }

    pub fn ring_len(&self) -> u32 {
        self.ring_len.load(Ordering::Relaxed)
    }

    pub fn capacity(&self) -> u32 {
        self.capacity.load(Ordering::Relaxed)
    }

    pub fn warmup_needed(&self) -> u32 {
        self.warmup_needed.load(Ordering::Relaxed)
    }

    // Frames, not milliseconds. The adaptive buffer's capacity fields are declared in
    // milliseconds but compared against this count, which is the unit confusion recorded
    // against the buffer redesign; the conversion is done here at read time instead.
    pub fn buffer_ms(&self) -> u32 {
        self.ring_len() * FRAME_DURATION_MS as u32
    }

    // How much of what played for this speaker was fabricated rather than decoded.
    //
    // This is what a listener actually experiences, and unlike a loss percentage it is derivable
    // from what this client can see. Concealment rises for a bad network and also while a speaker
    // is simply quiet, which is exactly the pair #232 wants separated — the separation comes from
    // reading it alongside the drop counters, not from this number alone.
    pub fn concealment_pct(&self) -> f32 {
        let decoded = self.frames_decoded();
        let concealed = self.plc_frames() + self.silence_frames();
        let total = decoded + concealed;
        if total == 0 {
            return 0.0;
        }
        ((concealed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32
    }

    // Mirrors the ratio the jitter buffer's own quality score uses, so the published figure
    // and the logged one cannot disagree.
    pub fn quality_score(&self) -> f64 {
        let decoded = self.frames_decoded();
        let plc = self.plc_frames();
        let silence = self.silence_frames();
        let total = decoded + plc + silence;
        if total == 0 {
            return 1.0;
        }

        let success = decoded as f64 / total as f64;
        let plc_penalty = (plc as f64 / total as f64) * 0.5;
        let silence_penalty = (silence as f64 / total as f64) * 0.8;
        (success - plc_penalty - silence_penalty).clamp(0.0, 1.0)
    }

    // True when nothing has arrived, used to keep an idle speaker out of the log and the
    // rollup: an eight-player server must not write eight lines every interval.
    pub fn is_idle(&self) -> bool {
        self.frames_received() == 0
    }

    pub fn to_diagnostics(&self) -> PeerDiagnostics {
        PeerDiagnostics {
            name: self.name.clone(),
            underruns: self.underruns(),
            overflow_drops: self.overflow_drops(),
            ooo_drops: self.ooo_drops(),
            plc_frames: self.plc_frames(),
            silence_frames: self.silence_frames(),
            frames_decoded: self.frames_decoded(),
            ring_len: self.ring_len(),
            capacity: self.capacity(),
            warmup_needed: self.warmup_needed(),
            quality_score: self.quality_score(),
            concealment_pct: self.concealment_pct(),
            buffer_ms: self.buffer_ms(),
        }
    }

    // Folds a second route's counters into a diagnostics record for the same speaker. A
    // player heard both normally and spatially has two jitter buffers, and reporting them as
    // two peers would double-count one speaker's drops.
    pub fn merge_into(&self, base: &mut PeerDiagnostics) {
        base.underruns += self.underruns();
        base.overflow_drops += self.overflow_drops();
        base.ooo_drops += self.ooo_drops();
        base.plc_frames += self.plc_frames();
        base.silence_frames += self.silence_frames();
        base.frames_decoded += self.frames_decoded();
        base.ring_len = base.ring_len.max(self.ring_len());
        base.capacity = base.capacity.max(self.capacity());
        base.warmup_needed = base.warmup_needed.max(self.warmup_needed());
        base.buffer_ms = base.buffer_ms.max(self.buffer_ms());
        base.concealment_pct = base.concealment_pct.max(self.concealment_pct());
        base.quality_score = base.quality_score.min(self.quality_score());
    }
}
