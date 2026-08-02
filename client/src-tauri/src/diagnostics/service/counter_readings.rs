use std::time::Instant;

// Previous readings of the monotonic counters, so the service can turn them into rates. Every
// producer only ever increments; all windowing happens here.
//
// Fields are `pub(super)` because the service both builds a reading and diffs against the previous
// one field by field. Nothing outside that module observes this type.
#[derive(Debug, Default, Clone)]
pub(super) struct CounterReadings {
    pub(super) at: Option<Instant>,
    pub(super) datagrams_sent: u64,
    pub(super) datagrams_received: u64,
    pub(super) frames_with_signal: u64,
    pub(super) packets_sent: u64,
    pub(super) packets_received: u64,
    pub(super) packets_lost: u64,
    pub(super) sequence_received: u64,
    pub(super) sequence_lost: u64,
    pub(super) burst_loss: u64,
}
