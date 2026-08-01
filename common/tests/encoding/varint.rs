use common::encoding::Varint;

// `AudioFramePacket`'s encoded length and timestamp are on the wire between every client and server,
// so this encoding is a cross-version contract rather than an implementation detail. These bytes were
// captured from the pre-refactor implementation; a change here is a protocol change.
//
// Round-trip tests are deliberately absent — they would exercise `leb128` and `residua_zigzag`, not
// this crate. Rule 10 bans that, and carves out exactly this case: a derivation whose stability is a
// cross-version contract.

#[test]
fn a_signed_timestamp_encodes_to_its_established_bytes() {
    assert_eq!(
        Varint::encode(1_700_000_000_000i64),
        vec![128, 160, 171, 254, 249, 98]
    );
}

#[test]
fn a_signed_length_encodes_to_its_established_bytes() {
    assert_eq!(Varint::encode(160i32), vec![192, 2]);
}

#[test]
fn signed_zero_and_negative_one_keep_their_zigzag_encoding() {
    // Zigzag is why -1 is one byte rather than five. Losing it would silently inflate every
    // negative-valued field on the wire.
    assert_eq!(Varint::encode(0i32), vec![0]);
    assert_eq!(Varint::encode(-1i32), vec![1]);
}

#[test]
fn unsigned_values_are_not_zigzag_mapped() {
    // 300 encodes as 300, not as 600. An unsigned value that went through zigzag would double and
    // cost an extra byte at every boundary.
    assert_eq!(Varint::encode(300u32), vec![172, 2]);
    assert_eq!(Varint::encode(1u64 << 40), vec![128, 128, 128, 128, 128, 32]);
}

#[test]
fn decode_reports_the_bytes_it_consumed() {
    // Callers advance through packed fields by this count, so it is part of the contract.
    let encoded = Varint::encode(160i32);
    let (value, consumed) = Varint::decode::<i32>(&encoded).expect("decodes");
    assert_eq!(value, 160);
    assert_eq!(consumed, encoded.len());
}

#[test]
fn a_truncated_varint_is_an_error_not_a_panic() {
    // A continuation bit with nothing following it, and an empty slice. Both arrive from the network.
    assert!(Varint::decode::<i64>(&[0x80]).is_err());
    assert!(Varint::decode::<i32>(&[]).is_err());
}

// Size budgets, carried over from the pre-refactor tests. These pin a datagram-budget decision
// rather than the codec: a 1150-byte cap makes per-field width a real constraint, and the whole
// reason for varint here is that these two fields stay small.

#[test]
fn typical_audio_frame_lengths_encode_within_two_bytes() {
    for length in [120, 240, 480, 960] {
        let encoded = Varint::encode(length as i32);
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

    let encoded = Varint::encode(now);
    assert!(
        encoded.len() <= 6,
        "timestamp {now} encoded to {} bytes (expected <= 6, vs 8 for a raw i64)",
        encoded.len()
    );
}
