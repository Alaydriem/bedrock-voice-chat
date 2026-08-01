use anyhow::Error;
use residua_zigzag::{ZigZagDecode, ZigZagEncode};
use std::io::Cursor;

// Variable-length encoding for one numeric type.
//
// Implemented per type rather than as a single generic function because the signed and unsigned
// paths genuinely differ: signed values are zigzag-mapped first so that small negatives stay short,
// while unsigned values must not be, since zigzag doubles a non-negative magnitude and would cost a
// byte at every varint boundary.
pub trait VarintValue: Sized {
    fn encode_varint(self) -> Vec<u8>;

    // Returns the value and the number of bytes consumed, so a caller reading packed fields can
    // advance past this one.
    fn decode_varint(data: &[u8]) -> Result<(Self, usize), Error>;
}

impl VarintValue for u64 {
    fn encode_varint(self) -> Vec<u8> {
        let mut buf = Vec::new();
        leb128::write::unsigned(&mut buf, self).expect("writing to a Vec cannot fail");
        buf
    }

    fn decode_varint(data: &[u8]) -> Result<(Self, usize), Error> {
        let mut reader = Cursor::new(data);
        let value = leb128::read::unsigned(&mut reader)?;
        Ok((value, reader.position() as usize))
    }
}

impl VarintValue for u32 {
    fn encode_varint(self) -> Vec<u8> {
        (self as u64).encode_varint()
    }

    fn decode_varint(data: &[u8]) -> Result<(Self, usize), Error> {
        let (value, consumed) = u64::decode_varint(data)?;
        Ok((value as u32, consumed))
    }
}

impl VarintValue for i64 {
    fn encode_varint(self) -> Vec<u8> {
        self.zigzag_encode().encode_varint()
    }

    fn decode_varint(data: &[u8]) -> Result<(Self, usize), Error> {
        let (zigzag, consumed) = u64::decode_varint(data)?;
        Ok((zigzag.zigzag_decode(), consumed))
    }
}

impl VarintValue for i32 {
    fn encode_varint(self) -> Vec<u8> {
        self.zigzag_encode().encode_varint()
    }

    fn decode_varint(data: &[u8]) -> Result<(Self, usize), Error> {
        let (zigzag, consumed) = u32::decode_varint(data)?;
        Ok((zigzag.zigzag_decode(), consumed))
    }
}
