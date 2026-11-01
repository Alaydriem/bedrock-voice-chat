use common::structs::relay::enroll::{EnrollFrame, EnrollRefuseReason, EnrollVersion};
use common::structs::relay::wire::Framing;

// The variant order is a cross-version contract: postcard encodes a variant as its
// index, so an insert anywhere but the end silently mis-decodes every later frame
// against a peer built from a different order. Pinning the first byte is what makes
// that a test failure rather than a field mis-read in production.
#[test]
fn hello_is_the_zeroth_variant() {
    let bytes = Framing::encode(&EnrollFrame::Hello {
        versions: vec![EnrollVersion(1)],
    })
    .expect("a hello frame encodes");

    assert_eq!(bytes[Framing::HEADER_LEN], 0);
}

#[test]
fn refuse_keeps_its_version_one_index_and_check_in_frames_follow() {
    let index = |frame: EnrollFrame| {
        Framing::encode(&frame).expect("a frame encodes")[Framing::HEADER_LEN]
    };

    assert_eq!(
        index(EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NotEntitled,
        }),
        9
    );
    assert_eq!(
        index(EnrollFrame::CheckIn {
            address: None,
            port: 443,
        }),
        10
    );
    assert_eq!(index(EnrollFrame::CheckedIn { nonce: vec![1] }), 11);
}

// Negotiation refuses rather than falling back. A session established on a version
// one side does not speak fails later, mid-exchange, instead of at connect where it
// can be reported.
#[test]
fn negotiation_returns_none_when_no_version_is_shared() {
    assert_eq!(
        EnrollVersion::negotiate(&[EnrollVersion(1)], &[EnrollVersion(2)]),
        None
    );
}

#[test]
fn negotiation_picks_the_highest_shared_version() {
    assert_eq!(
        EnrollVersion::negotiate(
            &[EnrollVersion(1), EnrollVersion(2)],
            &[EnrollVersion(2), EnrollVersion(3)]
        ),
        Some(EnrollVersion(2))
    );
}

// Captured from the encoder before the play frames were appended. A change here
// means a deployed peer would mis-decode the frame.
#[test]
fn existing_frames_keep_their_encoding() {
    let bytes = Framing::encode(&EnrollFrame::CheckIn {
        address: Some("1.2.3.4".into()),
        port: 443,
    })
    .expect("a check-in frame encodes");

    assert_eq!(
        bytes,
        vec![0, 0, 0, 12, 10, 1, 7, b'1', b'.', b'2', b'.', b'3', b'.', b'4', 0xbb, 0x03]
    );
}
