use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

// Transport measurements published outward from the QUIC event thread.
//
// Every field is an atomic because the writer is s2n-quic's event loop, which must not be
// made to take a lock or allocate. Counters are monotonic; rates and deltas are derived by
// the reader so the writer stays trivially correct.
//
// `Relaxed` throughout: these are observational, never used to order other work.
#[derive(Debug, Default)]
pub struct QuicLinkStats {
    smoothed_rtt_us: AtomicU32,
    latest_rtt_us: AtomicU32,
    min_rtt_us: AtomicU32,
    rtt_variance_us: AtomicU32,
    packets_sent: AtomicU64,
    packets_received: AtomicU64,
    packets_lost: AtomicU64,
    datagrams_dropped: AtomicU64,
    paths_used: AtomicU32,
    // Highest 1-RTT packet number observed, and the gap accounting derived from it.
    //
    // A QUIC receiver sees the sender's packet numbers and they are never reused, so a number that
    // never arrives was sent and lost. Two things stop that being a loss rate on its own:
    //
    // The peer deliberately skips single packet numbers — roughly once per congestion window, for
    // optimistic-ACK mitigation and PTO probes. The skip event fires on the *sender*, so a receiver
    // cannot identify them. A skip is always exactly one number, which is what makes runs of two or
    // more unambiguously loss and isolated gaps ambiguous.
    //
    // Reordering also opens a gap that a later packet fills, so arrivals below the high-water mark
    // are counted and subtracted rather than treated as loss.
    highest_packet_number: AtomicU64,
    seen_packet_number: AtomicU32,
    burst_loss: AtomicU64,
    isolated_gaps: AtomicU64,
    late_arrivals: AtomicU64,
    // Envelope sequence accounting, kept separate from the QUIC packet-number counters above.
    //
    // These are the authoritative downlink loss figures. The server assigns this sequence per
    // connection at the moment it queues a datagram, and nothing skips it — so unlike a QUIC packet
    // number, a single missing value here IS loss rather than possibly a deliberate skip.
    highest_sequence: AtomicU32,
    seen_sequence: AtomicU32,
    sequence_received: AtomicU64,
    sequence_lost: AtomicU64,
    sequence_late: AtomicU64,
    // Set once any RTT has been observed, so a reader can tell "no measurement yet" from a
    // genuine zero.
    rtt_seen: AtomicU32,
}

impl QuicLinkStats {
    pub fn new() -> Self {
        Self::default()
    }

    // The active-path filter lives here rather than in the event subscriber because
    // s2n-quic's event structs are `#[non_exhaustive]` and cannot be constructed outside
    // their crate — a filter written inside the subscriber would be untestable. The
    // subscriber reads one field and delegates; the policy is here, where it can be driven
    // directly.
    //
    // Discarding inactive paths matters because a dual-stack socket, permitted migration, and
    // a proxy's socket churn together mean measurements arrive for paths no longer carrying
    // traffic. Keeping the last one to fire would report a dead path's round trip as live.
    pub fn record_rtt_for_path(
        &self,
        is_active: bool,
        smoothed: Duration,
        latest: Duration,
        min: Duration,
        variance: Duration,
    ) {
        if !is_active {
            return;
        }
        self.record_rtt(smoothed, latest, min, variance);
    }

    pub fn record_rtt(
        &self,
        smoothed: Duration,
        latest: Duration,
        min: Duration,
        variance: Duration,
    ) {
        self.smoothed_rtt_us
            .store(Self::micros(smoothed), Ordering::Relaxed);
        self.latest_rtt_us
            .store(Self::micros(latest), Ordering::Relaxed);
        self.min_rtt_us.store(Self::micros(min), Ordering::Relaxed);
        self.rtt_variance_us
            .store(Self::micros(variance), Ordering::Relaxed);
        self.rtt_seen.store(1, Ordering::Relaxed);
    }

    pub fn record_sent(&self) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    // Every QUIC packet arriving from the peer, acknowledgements included. This is the counter a
    // stall is measured against: application datagrams stop whenever nobody is speaking, but a
    // live connection never stops acknowledging.
    pub fn record_received(&self) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_lost(&self) {
        self.packets_lost.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_datagram_dropped(&self) {
        self.datagrams_dropped.fetch_add(1, Ordering::Relaxed);
    }

    // Records a 1-RTT packet number and folds it into the gap accounting.
    //
    // Only the 1-RTT space may be passed here: Initial and Handshake number independently, and
    // mixing spaces would read as enormous loss at the transition.
    pub fn record_packet_number(&self, number: u64) {
        if self.seen_packet_number.swap(1, Ordering::Relaxed) == 0 {
            // The first number seen establishes the mark. Counting everything below it would report
            // the whole handshake as lost.
            self.highest_packet_number.store(number, Ordering::Relaxed);
            return;
        }

        let previous = self.highest_packet_number.load(Ordering::Relaxed);

        if number <= previous {
            // Reordered or duplicated. Either way a gap counted earlier was not loss.
            self.late_arrivals.fetch_add(1, Ordering::Relaxed);
            return;
        }

        self.highest_packet_number.store(number, Ordering::Relaxed);

        match number - previous - 1 {
            0 => {}
            // Exactly one missing: a deliberate skip or a single real loss, indistinguishable here.
            1 => {
                self.isolated_gaps.fetch_add(1, Ordering::Relaxed);
            }
            // Two or more consecutive: no skip mechanism produces this, so it is loss.
            missing => {
                self.burst_loss.fetch_add(missing, Ordering::Relaxed);
            }
        }
    }

    // A backward jump larger than half the range is the sender's counter wrapping rather than
    // reordering. Unreachable at any realistic session length — roughly 2.7 years of continuous
    // transmission — but misclassifying it would silently zero the loss figure, so it is stated.
    const SEQUENCE_ROLLOVER_THRESHOLD: u32 = u32::MAX / 2;

    // Records an envelope sequence observed from the peer and folds it into the loss accounting.
    pub fn record_sequence(&self, sequence: u32) {
        self.sequence_received.fetch_add(1, Ordering::Relaxed);

        if self.seen_sequence.swap(1, Ordering::Relaxed) == 0 {
            // The first value seen is the baseline. Counting everything below it would report every
            // packet sent before the diagnostics attached as lost.
            self.highest_sequence.store(sequence, Ordering::Relaxed);
            return;
        }

        let previous = self.highest_sequence.load(Ordering::Relaxed);

        if sequence > previous {
            let missing = sequence - previous - 1;
            if missing > 0 {
                self.sequence_lost.fetch_add(missing as u64, Ordering::Relaxed);
            }
            self.highest_sequence.store(sequence, Ordering::Relaxed);
            return;
        }

        if previous - sequence > Self::SEQUENCE_ROLLOVER_THRESHOLD {
            // Wrapped. Re-baseline rather than reporting four billion packets lost.
            self.highest_sequence.store(sequence, Ordering::Relaxed);
            return;
        }

        // Reordered or duplicated, so a gap counted earlier was not loss after all.
        self.sequence_late.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_path(&self) {
        self.paths_used.fetch_add(1, Ordering::Relaxed);
    }

    pub fn smoothed_rtt_ms(&self) -> Option<u32> {
        self.rtt_if_seen(&self.smoothed_rtt_us)
    }

    pub fn latest_rtt_ms(&self) -> Option<u32> {
        self.rtt_if_seen(&self.latest_rtt_us)
    }

    pub fn min_rtt_ms(&self) -> Option<u32> {
        self.rtt_if_seen(&self.min_rtt_us)
    }

    pub fn rtt_variance_ms(&self) -> Option<u32> {
        self.rtt_if_seen(&self.rtt_variance_us)
    }

    pub fn packets_sent(&self) -> u64 {
        self.packets_sent.load(Ordering::Relaxed)
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received.load(Ordering::Relaxed)
    }

    pub fn packets_lost(&self) -> u64 {
        self.packets_lost.load(Ordering::Relaxed)
    }

    pub fn datagrams_dropped(&self) -> u64 {
        self.datagrams_dropped.load(Ordering::Relaxed)
    }

    // Provable downlink loss: runs of two or more consecutive missing packet numbers, less the
    // reordered arrivals that turned out to fill a counted gap.
    //
    // A lower bound, not a rate. Isolated single losses are excluded because they are
    // indistinguishable from the peer's deliberate skips, so this under-reports rather than
    // inventing a phantom floor of 0.7-2.9% on a healthy link.
    pub fn burst_loss(&self) -> u64 {
        self.burst_loss
            .load(Ordering::Relaxed)
            .saturating_sub(self.late_arrivals.load(Ordering::Relaxed))
    }

    // The ambiguous population, surfaced rather than hidden: at a known healthy skip rate its size
    // is still informative.
    pub fn isolated_gaps(&self) -> u64 {
        self.isolated_gaps.load(Ordering::Relaxed)
    }

    pub fn late_arrivals(&self) -> u64 {
        self.late_arrivals.load(Ordering::Relaxed)
    }

    // Downlink loss as the server's own sequence reports it, net of reordering.
    //
    // `None` until a stamped envelope has arrived, so a peer predating the sequence field reads as
    // unmeasured rather than as a flawless link.
    pub fn downlink_loss(&self) -> Option<(u64, u64)> {
        if self.seen_sequence.load(Ordering::Relaxed) == 0 {
            return None;
        }

        let lost = self
            .sequence_lost
            .load(Ordering::Relaxed)
            .saturating_sub(self.sequence_late.load(Ordering::Relaxed));
        let received = self.sequence_received.load(Ordering::Relaxed);

        Some((lost, received))
    }

    pub fn paths_used(&self) -> u32 {
        self.paths_used.load(Ordering::Relaxed)
    }

    fn rtt_if_seen(&self, field: &AtomicU32) -> Option<u32> {
        if self.rtt_seen.load(Ordering::Relaxed) == 0 {
            return None;
        }
        Some(field.load(Ordering::Relaxed) / 1_000)
    }

    fn micros(value: Duration) -> u32 {
        value.as_micros().min(u32::MAX as u128) as u32
    }
}
