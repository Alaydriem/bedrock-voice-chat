use anyhow::Error;
use residua_zigzag::{ZigZagDecode, ZigZagEncode};
use std::io::Cursor;

/// Encode a u32 value using variable-length encoding (LEB128)
pub fn encode_varint_u32(value: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    leb128::write::unsigned(&mut buf, value as u64).unwrap();
    buf
}

/// Decode a u32 value from variable-length encoding (LEB128)
/// Returns (value, bytes_consumed)
pub fn decode_varint_u32(data: &[u8]) -> Result<(u32, usize), Error> {
    let mut reader = Cursor::new(data);
    let value = leb128::read::unsigned(&mut reader)? as u32;
    let bytes_read = reader.position() as usize;
    Ok((value, bytes_read))
}

/// Encode a u64 value using variable-length encoding (LEB128)
pub fn encode_varint_u64(value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    leb128::write::unsigned(&mut buf, value).unwrap();
    buf
}

/// Decode a u64 value from variable-length encoding (LEB128)
/// Returns (value, bytes_consumed)
pub fn decode_varint_u64(data: &[u8]) -> Result<(u64, usize), Error> {
    let mut reader = Cursor::new(data);
    let value = leb128::read::unsigned(&mut reader)?;
    let bytes_read = reader.position() as usize;
    Ok((value, bytes_read))
}

/// Encode an i32 value using zigzag encoding + variable-length encoding
/// This is efficient for small positive/negative numbers
pub fn encode_zigzag_varint_i32(value: i32) -> Vec<u8> {
    let zigzag = value.zigzag_encode();
    encode_varint_u32(zigzag)
}

/// Decode an i32 value from zigzag + variable-length encoding
/// Returns (value, bytes_consumed)
pub fn decode_zigzag_varint_i32(data: &[u8]) -> Result<(i32, usize), Error> {
    let (zigzag, size) = decode_varint_u32(data)?;
    let value = zigzag.zigzag_decode();
    Ok((value, size))
}

/// Encode an i64 value using zigzag encoding + variable-length encoding
/// This is efficient for timestamps and other values close to zero
pub fn encode_zigzag_varint_i64(value: i64) -> Vec<u8> {
    let zigzag = value.zigzag_encode();
    encode_varint_u64(zigzag)
}

/// Decode an i64 value from zigzag + variable-length encoding
/// Returns (value, bytes_consumed)
pub fn decode_zigzag_varint_i64(data: &[u8]) -> Result<(i64, usize), Error> {
    let (zigzag, size) = decode_varint_u64(data)?;
    let value = zigzag.zigzag_decode();
    Ok((value, size))
}
