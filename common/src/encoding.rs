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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_u32() {
        let test_values = [0, 1, 127, 128, 255, 256, 65535, 65536];

        for value in test_values {
            let encoded = encode_varint_u32(value);
            let (decoded, size) = decode_varint_u32(&encoded).unwrap();
            assert_eq!(value, decoded);
            assert_eq!(size, encoded.len());
        }
    }

    #[test]
    fn test_zigzag_varint_i32() {
        let test_values = [0, -1, 1, -64, 64, -128, 128, i32::MIN, i32::MAX];

        for value in test_values {
            let encoded = encode_zigzag_varint_i32(value);
            let (decoded, size) = decode_zigzag_varint_i32(&encoded).unwrap();
            assert_eq!(value, decoded);
            assert_eq!(size, encoded.len());
        }
    }

    #[test]
    fn test_zigzag_varint_i64() {
        let test_values = [0, -1, 1, -64, 64, -128, 128, i64::MIN, i64::MAX];

        for value in test_values {
            let encoded = encode_zigzag_varint_i64(value);
            let (decoded, size) = decode_zigzag_varint_i64(&encoded).unwrap();
            assert_eq!(value, decoded);
            assert_eq!(size, encoded.len());
        }
    }

    #[test]
    fn test_space_savings() {
        // Test typical audio frame lengths (120-960 bytes)
        let typical_lengths = [120, 240, 480, 960];

        for length in typical_lengths {
            let encoded = encode_zigzag_varint_i32(length);
            println!(
                "Length {} encodes to {} bytes (vs 4 bytes for i32)",
                length,
                encoded.len()
            );
            assert!(encoded.len() <= 2); // Should be 1-2 bytes for typical values
        }

        // Test recent timestamps (should encode efficiently)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let encoded = encode_zigzag_varint_i64(now);
        println!(
            "Timestamp {} encodes to {} bytes (vs 8 bytes for i64)",
            now,
            encoded.len()
        );
    }

    #[test]
    fn test_audio_frame_packet_creation() {
        use crate::structs::packet::AudioFramePacket;
        use crate::Coordinate;
        use crate::game_data::Dimension;

        // Create a test audio frame packet
        let test_data = vec![1, 2, 3, 4, 5];
        let packet = AudioFramePacket::new(
            test_data.clone(),
            48000,
            Some(Coordinate {
                x: 100.0,
                y: 50.0,
                z: 200.0,
            }),
            None,
            Some(Dimension::Overworld),
            Some(true),
        );

        // Verify the accessors work
        assert_eq!(packet.length(), test_data.len() as i32);
        assert_eq!(packet.data_len(), test_data.len());
        assert_eq!(packet.sample_rate, 48000);
        assert_eq!(packet.data, test_data);

        // Verify timestamp is recent (within last minute)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let packet_time = packet.timestamp();
        assert!((now - packet_time).abs() < 60_000); // Within 60 seconds

        println!(
            "AudioFramePacket created successfully with length {} and timestamp {}",
            packet.length(),
            packet.timestamp()
        );
    }

    #[test]
    fn test_packet_serialization_roundtrip() {
        use crate::structs::packet::{
            AudioFramePacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
        };
        use crate::Coordinate;
        use crate::game_data::Dimension;

        // Create a test audio frame
        let test_data = vec![0x01, 0x02, 0x03, 0x04]; // Small test audio data
        let audio_packet = AudioFramePacket::new(
            test_data.clone(),
            48000,
            Some(Coordinate {
                x: 100.0,
                y: 50.0,
                z: 200.0,
            }),
            None,
            Some(Dimension::Overworld),
            Some(true),
        );

        // Create a complete QUIC packet
        let original_packet = QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: Some(PacketOwner {
                name: "test_user".to_string(),
                client_id: vec![1, 2, 3, 4],
            }),
            data: QuicNetworkPacketData::AudioFrame(audio_packet),
        };

        // Serialize the packet (datagram form)
        let serialized = original_packet
            .to_datagram()
            .expect("Failed to serialize packet");
        let serialized_len = serialized.len();
        println!("Datagram serialized to {} bytes", serialized_len);

        // Deserialize the packet directly from datagram bytes
        let deserialized_packet = QuicNetworkPacket::from_datagram(&serialized)
            .expect("Failed to deserialize datagram packet");

        // Verify packet type and owner
        assert_eq!(deserialized_packet.packet_type, PacketType::AudioFrame);
        assert_eq!(deserialized_packet.get_author(), "test_user");

        // Verify audio data
        if let QuicNetworkPacketData::AudioFrame(audio) = &deserialized_packet.data {
            assert_eq!(audio.length(), test_data.len() as i32);
            assert_eq!(audio.data, test_data);
            assert_eq!(audio.sample_rate, 48000);
            println!(
                "Round-trip successful! Deserialized length: {}, timestamp: {}",
                audio.length(),
                audio.timestamp()
            );
        } else {
            panic!("Deserialized packet is not an AudioFrame");
        }

        // Test space efficiency of our optimized fields
        if let QuicNetworkPacketData::AudioFrame(audio) = &deserialized_packet.data {
            let encoded_length_size = audio.encoded_length_size();
            let encoded_timestamp_size = audio.encoded_timestamp_size();

            println!(
                "Encoded length field: {} bytes (vs 4 bytes for u32 or 8 for usize)",
                encoded_length_size
            );
            println!(
                "Encoded timestamp field: {} bytes (vs 8 bytes for i64)",
                encoded_timestamp_size
            );
            println!(
                "Total AudioFramePacket field savings: {} bytes per packet",
                (4 + 8) - (encoded_length_size + encoded_timestamp_size)
            );
        }

        // Datagram path removed all custom stream framing overhead (previous header bytes eliminated)
        println!("Datagram path removes custom header; size now = payload only.");
    }
}
