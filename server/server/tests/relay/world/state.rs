use std::time::Duration;

use bvc_server_lib::relay::WorldWatchState;

fn worlds(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn configured(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(label, world)| (label.to_string(), world.to_string()))
        .collect()
}

// The first observation is a change: a server that starts with players already
// in a world would otherwise never log the set it is hosting.
#[test]
fn the_first_observation_is_a_change() {
    let mut state = WorldWatchState::new();

    assert_eq!(
        state.observe(&worlds(&["overworld"])),
        Some(worlds(&["overworld"]))
    );
}

#[test]
fn an_unchanged_set_is_not_a_change() {
    let mut state = WorldWatchState::new();
    state.observe(&worlds(&["overworld"]));

    assert_eq!(state.observe(&worlds(&["overworld"])), None);
}

// The cache hands back a sorted list today, but the set is what changed, not the
// order. Reporting a reordering as a change would put a line in the log on every
// tick of a server hosting the same two worlds.
#[test]
fn reordering_is_not_a_change() {
    let mut state = WorldWatchState::new();
    state.observe(&worlds(&["nether", "overworld"]));

    assert_eq!(state.observe(&worlds(&["overworld", "nether"])), None);
}

#[test]
fn emptying_is_a_change() {
    let mut state = WorldWatchState::new();
    state.observe(&worlds(&["overworld"]));

    assert_eq!(state.observe(&[]), Some(Vec::new()));
}

// Before the grace period the server has seen nothing, so every configured world
// looks missing. Warning then would fire on every correctly configured server
// that simply had not started yet.
#[test]
fn nothing_is_warned_before_the_grace_period() {
    let mut state = WorldWatchState::new();

    assert!(
        state
            .unwarned_missing(&configured(&[("svc-bridge", "overworld")]), Duration::ZERO)
            .is_empty()
    );
}

#[test]
fn a_world_never_observed_is_warned_once() {
    let mut state = WorldWatchState::new();
    let cfg = configured(&[("svc-bridge", "overworld")]);
    let after = WorldWatchState::GRACE + Duration::from_secs(1);

    assert_eq!(state.unwarned_missing(&cfg, after), cfg);
    assert!(state.unwarned_missing(&cfg, after).is_empty());
}

// A world that emptied out was still real. Warning on it because nobody is in it
// right now would tell an operator their config is wrong when it is right.
#[test]
fn a_world_observed_once_is_never_warned() {
    let mut state = WorldWatchState::new();
    state.observe(&worlds(&["overworld"]));
    state.observe(&[]);

    let after = WorldWatchState::GRACE + Duration::from_secs(1);
    assert!(
        state
            .unwarned_missing(&configured(&[("svc-bridge", "overworld")]), after)
            .is_empty()
    );
}
