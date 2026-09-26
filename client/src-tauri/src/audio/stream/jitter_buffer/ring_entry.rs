use super::EncodedAudioFramePacket;

/// A queued packet, and the arrival stamp it is recorded under when recording was on at arrival.
///
/// Carried together so a frame is recorded exactly when it decodes. A frame removed from the ring
/// without being decoded takes its recording with it, and a frame that fails to decode is never
/// recorded, because there is no second queue to fall out of step with the first.
pub(super) struct RingEntry {
    pub(super) packet: EncodedAudioFramePacket,
    pub(super) recorded_at_ms: Option<u64>,
}
