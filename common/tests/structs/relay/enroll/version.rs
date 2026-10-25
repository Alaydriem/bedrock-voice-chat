use common::structs::relay::enroll::EnrollVersion;

#[test]
fn a_version_one_peer_shares_no_version() {
    assert_eq!(
        EnrollVersion::negotiate(EnrollVersion::SUPPORTED, &[EnrollVersion(1)]),
        None
    );
}
