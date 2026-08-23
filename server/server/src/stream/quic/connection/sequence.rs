use common::curia;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use common::structs::packet::QuicNetworkPacket;

// The per-connection outbound sequence.
//
// Exists so the one invariant this mechanism rests on has a single enforcement point: a number is
// consumed only when a datagram is actually produced for this connection. Every routing decision
// not to send — proximity, channel membership, deafen distance, a serialization failure — must
// happen before `stamp` is called, or the receiver sees a gap that was never loss and reports
// phantom packet loss.
//
// `Relaxed` is sufficient: the counter orders nothing else, and the receiver only cares that values
// are distinct and ascending, which `fetch_add` guarantees regardless of ordering.
#[derive(Debug, Default)]
pub struct ConnectionSequence {
    next: AtomicU32,
}

impl ConnectionSequence {
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    // Stamps `packet` for this connection and serializes it.
    //
    // For a delivery with a single recipient, where there is no second listener to amortise an
    // encode across. Fan-out uses `patch` instead.
    //
    // Wraps at `u32::MAX`, which at ~50 datagrams/s is roughly 2.7 years on a single connection —
    // unreachable in practice, and the receiver re-baselines on a large backward jump regardless.
    pub fn stamp(&self, packet: &mut QuicNetworkPacket) -> Option<Bytes> {
        let sequence = self.next.fetch_add(1, Ordering::Relaxed);
        packet.stamp(sequence);

        match packet.to_datagram() {
            Ok(bytes) => Some(Bytes::from(bytes)),
            // The number is already spent, so an oversized packet costs one phantom lost packet at
            // the receiver. That is preferable to the alternative — reserving after serializing
            // would let two concurrent senders issue the same number — and an envelope exceeding
            // the datagram cap is a bug that is logged where it is built.
            Err(e) => {
                curia::error!("failed to serialize stamped datagram: {}", e);
                None
            }
        }
    }

    // Copies an already-serialized envelope and rewrites only its sequence bytes, so one packet
    // going to N recipients is encoded once rather than N times.
    //
    // `template` must have been serialized with a sequence present, or the range written here is
    // simply not in it — an absent `Option` encodes as a bare tag and no value bytes. Patching
    // cannot change the length, so the datagram size ceiling still holds from the single encode
    // that produced the template.
    //
    // The number is spent before the copy, on the same reasoning as `stamp`.
    pub fn patch(&self, template: &[u8]) -> Option<Bytes> {
        let sequence = self.next.fetch_add(1, Ordering::Relaxed);
        let range = QuicNetworkPacket::SEQ_VALUE_RANGE;

        if template.len() < range.end {
            curia::error!(
                "envelope template of {} bytes is too short to carry a sequence",
                template.len()
            );
            return None;
        }

        let mut buf = template.to_vec();
        buf[range].copy_from_slice(&sequence.to_le_bytes());
        Some(Bytes::from(buf))
    }
}
