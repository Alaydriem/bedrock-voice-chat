use common::structs::relay::wire::WireVersion;

#[test]
fn negotiate_picks_the_highest_version_both_sides_speak() {
    let local = [WireVersion(1), WireVersion(2), WireVersion(3)];
    let remote = [WireVersion(2), WireVersion(3)];

    assert_eq!(
        WireVersion::negotiate(&local, &remote),
        Some(WireVersion(3))
    );
}

#[test]
fn negotiate_ignores_versions_only_one_side_speaks() {
    let local = [WireVersion(1), WireVersion(4)];
    let remote = [WireVersion(2), WireVersion(4), WireVersion(9)];

    assert_eq!(
        WireVersion::negotiate(&local, &remote),
        Some(WireVersion(4))
    );
}

#[test]
fn negotiate_yields_none_when_no_version_is_shared() {
    let local = [WireVersion(1), WireVersion(2)];
    let remote = [WireVersion(3)];

    assert_eq!(WireVersion::negotiate(&local, &remote), None);
}

#[test]
fn negotiate_yields_none_against_a_peer_offering_nothing() {
    assert_eq!(WireVersion::negotiate(WireVersion::SUPPORTED, &[]), None);
}

#[test]
fn this_build_negotiates_with_itself() {
    assert!(
        WireVersion::negotiate(WireVersion::SUPPORTED, WireVersion::SUPPORTED).is_some(),
        "a build must be able to peer with an identical build"
    );
}

#[test]
fn capability_tags_round_trip_through_their_config_spelling() {
    use common::structs::relay::Capability;

    for capability in [
        Capability::CarrySpeakers,
        Capability::QueryAudio,
        Capability::ServeAudio,
    ] {
        assert_eq!(Capability::from_tag(capability.as_str()), Some(capability));
    }
}

#[test]
fn an_unknown_capability_tag_is_rejected_rather_than_defaulted() {
    use common::structs::relay::Capability;

    assert_eq!(Capability::from_tag("carry_speaker"), None);
    assert_eq!(Capability::from_tag(""), None);
}
