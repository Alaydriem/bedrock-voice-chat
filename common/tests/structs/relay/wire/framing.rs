use common::structs::relay::wire::control::Hello;
use common::structs::relay::wire::{ControlFrame, Framing, WireVersion};

#[test]
fn a_framed_control_frame_carries_a_four_byte_big_endian_length() {
    let frame = ControlFrame::Hello(Hello {
        versions: vec![WireVersion(1)],
        worlds: Vec::new(),
    });

    let encoded = Framing::encode(&frame).expect("encode");

    // Hello encodes to four bytes, so the header states four.
    assert_eq!(&encoded[..4], &[0x00, 0x00, 0x00, 0x04]);
    assert_eq!(&encoded[4..], &[0x00, 0x01, 0x01, 0x00]);
}

#[test]
fn a_header_reports_the_payload_length_it_states() {
    assert_eq!(
        Framing::payload_len(&[0x00, 0x00, 0x00, 0x03]).expect("valid"),
        3
    );
}

// The length arrives from a peer before any of its payload does, so an absurd
// value must be refused rather than used to size an allocation.
#[test]
fn a_header_over_the_cap_is_refused_before_anything_is_allocated() {
    let too_big = (Framing::MAX_FRAME as u32 + 1).to_be_bytes();

    assert!(Framing::payload_len(&too_big).is_err());
}

#[test]
fn a_zero_length_header_is_refused() {
    assert!(Framing::payload_len(&[0, 0, 0, 0]).is_err());
}
