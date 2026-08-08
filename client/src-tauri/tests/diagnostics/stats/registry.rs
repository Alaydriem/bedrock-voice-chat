use std::sync::Arc;

use bvc_client_lib::diagnostics::{PeerRegistry, PeerRoute, PlayerReceiveStats};

// `peers()` omits idle records and `is_idle()` is `frames_received() == 0`, so a record only
// appears once something has arrived. `record_arrival` is what bumps that counter;
// `record_decode` bumps a different one and would leave the record idle.
fn heard(name: &str, frames: usize) -> Arc<PlayerReceiveStats> {
    let stats = Arc::new(PlayerReceiveStats::new(name.to_string()));
    for _ in 0..frames {
        stats.record_arrival(0);
    }
    stats.record_decode(frames);
    stats
}

// A jukebox key carries its playback's event id, so a second disc in the same block is a
// different speaker here. Left registered, every disc a session plays keeps a row forever.
#[test]
fn unregister_removes_both_routes_for_one_speaker() {
    let registry = PeerRegistry::new();
    let stats = heard("jukebox-60fc0bd7", 1);

    registry.register(
        "jukebox-60fc0bd7".to_string(),
        PeerRoute::Spatial,
        stats.clone(),
    );
    registry.register("jukebox-60fc0bd7".to_string(), PeerRoute::Normal, stats);
    assert_eq!(registry.peer_count(), 1);

    registry.unregister("jukebox-60fc0bd7");

    assert_eq!(registry.peer_count(), 0);
}

#[test]
fn unregister_leaves_other_speakers_registered() {
    let registry = PeerRegistry::new();

    registry.register(
        "jukebox-60fc0bd7".to_string(),
        PeerRoute::Spatial,
        heard("jukebox-60fc0bd7", 1),
    );
    registry.register(
        "minecraft:Alaydriem".to_string(),
        PeerRoute::Spatial,
        heard("Alaydriem", 1),
    );

    registry.unregister("jukebox-60fc0bd7");

    let peers = registry.peers();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].name, "Alaydriem");
}

// The sweep compares one pass against the next, so it needs counts keyed by sink key.
// `peers()` cannot serve: it merges by display name and drops idle records, and the sweep
// has to see a jukebox that arrived and then went quiet.
#[test]
fn jukebox_frame_counts_reports_only_jukebox_keys() {
    let registry = PeerRegistry::new();
    registry.register(
        "jukebox-60fc0bd7".to_string(),
        PeerRoute::Spatial,
        heard("jukebox-60fc0bd7", 3),
    );
    registry.register(
        "minecraft:Alaydriem".to_string(),
        PeerRoute::Spatial,
        heard("Alaydriem", 9),
    );

    let counts = registry.jukebox_frame_counts();

    assert_eq!(counts, vec![("jukebox-60fc0bd7".to_string(), 3)]);
}

// A jukebox heard both spatially and normally holds two entries for one sink. The sweep must
// see one row, and must not call a sink quiet because the route with fewer frames stalled.
#[test]
fn jukebox_frame_counts_folds_both_routes_taking_the_higher() {
    let registry = PeerRegistry::new();
    registry.register(
        "jukebox-60fc0bd7".to_string(),
        PeerRoute::Spatial,
        heard("jukebox-60fc0bd7", 7),
    );
    registry.register(
        "jukebox-60fc0bd7".to_string(),
        PeerRoute::Normal,
        heard("jukebox-60fc0bd7", 2),
    );

    let counts = registry.jukebox_frame_counts();

    assert_eq!(counts, vec![("jukebox-60fc0bd7".to_string(), 7)]);
}

// An entry that never received anything still has to be reported, or a playback that produced
// no frames at all — the exact failure this whole effort exists to catch — would leave its sink
// registered forever.
#[test]
fn jukebox_frame_counts_includes_a_sink_that_never_received() {
    let registry = PeerRegistry::new();
    registry.register(
        "jukebox-903a6c97".to_string(),
        PeerRoute::Spatial,
        Arc::new(PlayerReceiveStats::new("jukebox-903a6c97".to_string())),
    );

    assert_eq!(
        registry.jukebox_frame_counts(),
        vec![("jukebox-903a6c97".to_string(), 0)]
    );
}
