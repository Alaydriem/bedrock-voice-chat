use common::structs::relay::enroll::EnrollVersion;

#[test]
fn a_version_one_peer_shares_no_version() {
    assert_eq!(
        EnrollVersion::negotiate(EnrollVersion::SUPPORTED, &[EnrollVersion(1)]),
        None
    );
}

#[test]
fn version_three_is_chosen_when_both_sides_speak_it() {
    assert_eq!(
        EnrollVersion::negotiate(
            EnrollVersion::SUPPORTED,
            &[EnrollVersion(2), EnrollVersion(3)]
        ),
        Some(EnrollVersion(3))
    );
}

#[test]
fn a_version_two_peer_still_negotiates() {
    assert_eq!(
        EnrollVersion::negotiate(EnrollVersion::SUPPORTED, &[EnrollVersion(2)]),
        Some(EnrollVersion(2))
    );
}
