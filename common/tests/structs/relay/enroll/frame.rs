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
fn refuse_is_the_last_variant() {
    let bytes = Framing::encode(&EnrollFrame::Refuse {
        reason: EnrollRefuseReason::NotEntitled,
    })
    .expect("a refuse frame encodes");

    assert_eq!(bytes[Framing::HEADER_LEN], 9);
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
