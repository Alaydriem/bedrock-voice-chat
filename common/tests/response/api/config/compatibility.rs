use common::response::api::config::ProtocolCompatibility;

#[test]
fn the_same_major_and_minor_are_compatible() {
    let check = ProtocolCompatibility::between("2.1.0", "2.1.0");

    assert!(check.compatible);
    assert!(!check.client_too_old);
}

// Patch releases do not change the wire format, so they must not split a fleet.
#[test]
fn a_differing_patch_is_still_compatible() {
    assert!(ProtocolCompatibility::between("2.1.7", "2.1.0").compatible);
}

#[test]
fn a_behind_client_is_incompatible_and_says_which_side_is_behind() {
    let check = ProtocolCompatibility::between("2.2.0", "2.1.0");

    assert!(!check.compatible);
    assert!(check.client_too_old);
}

// The other direction is equally incompatible but not fixable by updating, so the
// two are distinguishable rather than collapsed into one failure.
#[test]
fn an_ahead_client_is_incompatible_without_being_too_old() {
    let check = ProtocolCompatibility::between("2.1.0", "2.2.0");

    assert!(!check.compatible);
    assert!(!check.client_too_old);
}

#[test]
fn a_differing_major_is_incompatible() {
    assert!(!ProtocolCompatibility::between("3.0.0", "2.1.0").compatible);
}

// An unparseable version reads as 0.0 rather than panicking: a server that answers
// with nonsense is a server we cannot talk to, which is the same conclusion.
#[test]
fn an_unparseable_version_is_treated_as_zero() {
    let check = ProtocolCompatibility::between("banana", "2.1.0");

    assert!(!check.compatible);
    assert!(!check.client_too_old);
}

#[test]
fn both_versions_are_reported_back_for_the_message() {
    let check = ProtocolCompatibility::between("2.2.0", "2.1.0");

    assert_eq!(check.server_version, "2.2.0");
    assert_eq!(check.client_version, "2.1.0");
}
