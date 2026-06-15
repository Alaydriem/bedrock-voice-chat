use common::encoding::{encode_zigzag_varint_i32, encode_zigzag_varint_i64};

#[test]
fn typical_audio_frame_lengths_encode_within_two_bytes() {
    let typical_lengths = [120, 240, 480, 960];

    for length in typical_lengths {
        let encoded = encode_zigzag_varint_i32(length);
        assert!(
            encoded.len() <= 2,
            "length {length} encoded to {} bytes (expected <= 2)",
            encoded.len()
        );
    }
}

#[test]
fn recent_timestamp_encodes_within_six_bytes() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let encoded = encode_zigzag_varint_i64(now);
    assert!(
        encoded.len() <= 6,
        "timestamp {now} encoded to {} bytes (expected <= 6, vs 8 for a raw i64)",
        encoded.len()
    );
}
