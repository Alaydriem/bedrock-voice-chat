mod outbound_decision;

pub use outbound_decision::OutboundDecision;

use std::sync::Arc;

use common::structs::packet::AudioFramePacket;

use crate::groups::protected::SealGate;

/// The audio pipeline's one decision about protected groups, made through the group's gate.
///
/// Outbound, a protected member's frame is sealed or not sent at all — never sent in the clear.
/// Inbound, a sealed frame opens only if the member whose key sealed it is the sender the server
/// named. Plain frames are proximity audio from nearby players and pass through, unless they claim
/// to come from a group member: members only ever send sealed frames, so such a frame is forged.
pub struct ProtectedFrameFilter {
    gate: Arc<SealGate>,
}

impl ProtectedFrameFilter {
    pub fn new(gate: Arc<SealGate>) -> Self {
        Self { gate }
    }

    pub fn is_protected(&self) -> bool {
        self.gate.is_protected()
    }

    pub fn outbound(&self, encoded: &[u8]) -> OutboundDecision {
        if !self.gate.is_protected() {
            return OutboundDecision::Plain(encoded.to_vec());
        }
        match self.gate.seal(encoded) {
            Some(sealed) => {
                #[cfg(feature = "e2e")]
                crate::testkit::WireTap::sealed_plaintext(encoded);
                OutboundDecision::Sealed(sealed)
            }
            None => OutboundDecision::Drop,
        }
    }

    /// The Opus to play, or `None` to drop the frame.
    pub fn inbound(&self, frame: &AudioFramePacket, sender: &str) -> Option<Vec<u8>> {
        if !frame.is_sealed() {
            if self.gate.is_protected() && self.gate.is_member(sender) {
                return None;
            }
            return Some(frame.data.to_vec());
        }
        self.gate.open(&frame.data, sender)
    }
}
