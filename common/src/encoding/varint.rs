use anyhow::Error;

use super::VarintValue;

// The single entry point for variable-length integer encoding.
//
// Wire-format critical: `AudioFramePacket`'s encoded length and timestamp cross every
// client/server boundary, so the per-type behaviour behind this is fixed and any change to it is a
// protocol change.
pub struct Varint;

impl Varint {
    pub fn encode<T: VarintValue>(value: T) -> Vec<u8> {
        value.encode_varint()
    }

    // Returns the value and the bytes consumed, so a caller decoding packed fields can advance.
    pub fn decode<T: VarintValue>(data: &[u8]) -> Result<(T, usize), Error> {
        T::decode_varint(data)
    }
}
