use bvc_client_lib::groups::{GroupError, GroupResolution};
use common::structs::channel::Channel;

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

fn channel(name: &str, players: &[&str]) -> Channel {
    let mut channel = Channel::new(name.to_string(), identity("minecraft:Owner"));
    for player in players {
        channel.add_player(identity(player));
    }
    channel
}

#[test]
fn finds_a_group_by_its_exact_name() {
    let channels = vec![channel("Ops", &[]), channel("Raid", &[])];

    let found = GroupResolution::by_name(&channels, "Ops").expect("Ops resolves");

    assert_eq!(found.name, "Ops");
}

// What an operator types on a controller does not carry the casing a group was created with, and
// refusing over casing alone would make the button unusable.
#[test]
fn falls_back_to_a_case_insensitive_match() {
    let channels = vec![channel("Ops", &[])];

    let found = GroupResolution::by_name(&channels, "ops").expect("ops resolves");

    assert_eq!(found.name, "Ops");
}

// An exact match wins over a case-insensitive one, so a deliberate distinction between two groups
// is honoured rather than resolved by whichever the server listed first.
#[test]
fn prefers_an_exact_match_over_a_case_insensitive_one() {
    let channels = vec![channel("ops", &[]), channel("Ops", &[])];

    let found = GroupResolution::by_name(&channels, "Ops").expect("Ops resolves");

    assert_eq!(found.name, "Ops");
}

#[test]
fn reports_a_name_that_matches_nothing() {
    let channels = vec![channel("Ops", &[])];

    match GroupResolution::by_name(&channels, "Raid") {
        Err(GroupError::NotFound(name)) => assert_eq!(name, "Raid"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

// `Channel` carries no creation time, so "the oldest" is not derivable. Picking one silently would
// put an operator in a group they did not mean, talking to people they did not expect.
#[test]
fn refuses_an_ambiguous_name_rather_than_guessing() {
    let channels = vec![channel("Ops", &[]), channel("Ops", &[])];

    match GroupResolution::by_name(&channels, "Ops") {
        Err(GroupError::Ambiguous { name, count }) => {
            assert_eq!(name, "Ops");
            assert_eq!(count, 2);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// Two groups differing only in case are two groups. Resolving a name that matches neither exactly
// is ambiguous for the same reason as two identical names.
#[test]
fn treats_a_case_insensitive_tie_as_ambiguous() {
    let channels = vec![channel("Ops", &[]), channel("OPS", &[])];

    match GroupResolution::by_name(&channels, "ops") {
        Err(GroupError::Ambiguous { count, .. }) => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// Membership is keyed `game:gamertag`. A lookup by bare gamertag matches nothing, which would make
// a leave a no-op that reports success — the operator stays in the group and is told they left.
#[test]
fn finds_the_group_a_player_is_in_by_canonical_key() {
    let channels = vec![
        channel("Ops", &["minecraft:Alice"]),
        channel("Raid", &["minecraft:Bob"]),
    ];

    let found = GroupResolution::containing(&channels, "minecraft:Bob").expect("Bob is in Raid");
    assert_eq!(found.name, "Raid");

    assert!(GroupResolution::containing(&channels, "Bob").is_none());
}

#[test]
fn finds_nothing_when_a_player_is_in_no_group() {
    let channels = vec![channel("Ops", &["minecraft:Alice"])];

    assert!(GroupResolution::containing(&channels, "minecraft:Bob").is_none());
}
