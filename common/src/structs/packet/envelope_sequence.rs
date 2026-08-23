use serde::{Deserialize, Serialize};

/// A per-connection envelope sequence whose encoded width does not depend on its value.
///
/// Postcard encodes an integer as a varint by default, so the same field occupies one byte at
/// zero and five near `u32::MAX`. The audio fan-out serializes one envelope per frame and
/// rewrites only these bytes for each recipient, which requires the field to occupy a constant
/// number of them. `QuicNetworkPacket::SEQ_VALUE_RANGE` is that range, and
/// `common/tests/structs/packet/envelope_sequence.rs` pins it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeSequence(#[serde(with = "postcard::fixint::le")] pub u32);
