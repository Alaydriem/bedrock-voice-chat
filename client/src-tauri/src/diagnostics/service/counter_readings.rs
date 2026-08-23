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
    // Audio frames only, told apart from `datagrams_sent`, which counts position, presence,
    // control and health traffic as well. A microphone rate taken from the latter reads as
    // healthy on a client whose capture stream is dead.
    pub(super) audio_frames_sent: u64,
    // Meter messages published to the webview. The cost this whole path is tuned against on
    // Android, where every message is a unit of main-thread work.
    pub(super) meter_events: u64,
    pub(super) frames_captured: u64,
    pub(super) frames_with_signal: u64,
    pub(super) packets_sent: u64,
    pub(super) packets_received: u64,
    pub(super) packets_lost: u64,
    pub(super) sequence_received: u64,
    pub(super) sequence_lost: u64,
    pub(super) burst_loss: u64,
}
