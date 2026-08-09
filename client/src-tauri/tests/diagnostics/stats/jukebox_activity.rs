use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::diagnostics::{PeerRegistry, PeerRoute, PlayerReceiveStats};

#[test]
fn nothing_has_arrived_on_a_fresh_registry() {
    let registry = PeerRegistry::new();

    assert!(!registry.jukebox_playing(PeerRegistry::JUKEBOX_PLAYING_WINDOW));
}

#[test]
fn a_noted_frame_means_music_is_playing() {
    let registry = PeerRegistry::new();

    registry.note_jukebox_frame();

    assert!(registry.jukebox_playing(PeerRegistry::JUKEBOX_PLAYING_WINDOW));
}

// "The disc ended" is the window elapsing, and the answer has to go false on its own — nothing
// unregisters a sink to announce it. Read across the boundary rather than sleeping over it.
#[test]
fn a_frame_stops_counting_once_the_window_has_elapsed() {
    const WINDOW: Duration = PeerRegistry::JUKEBOX_PLAYING_WINDOW;
    let elapsed = WINDOW.as_millis() as u64;
    let registry = PeerRegistry::new();

    registry.note_jukebox_frame_at(1_000);

    assert!(registry.jukebox_playing_at(1_000 + elapsed, WINDOW));
    assert!(!registry.jukebox_playing_at(1_000 + elapsed + 1, WINDOW));
}

// The whole point of a separate stamp: registered counters stop when a frame is muted, and this
// must not. Registering nothing at all and still reporting playing is the behaviour.
#[test]
fn arrivals_are_counted_without_any_sink_being_registered() {
    let registry = PeerRegistry::new();

    registry.note_jukebox_frame();

    assert_eq!(registry.peer_count(), 0);
    assert!(registry.jukebox_playing(PeerRegistry::JUKEBOX_PLAYING_WINDOW));
}

// Retiring a playback's sink is unrelated to whether music is arriving; the next disc's frames
// land before anything is registered for them.
#[test]
fn unregistering_a_sink_does_not_clear_the_arrival() {
    let registry = PeerRegistry::new();
    let stats = Arc::new(PlayerReceiveStats::new("jukebox-60fc0bd7".to_string()));
    registry.register("jukebox-60fc0bd7".to_string(), PeerRoute::Spatial, stats);
    registry.note_jukebox_frame();

    registry.unregister("jukebox-60fc0bd7");

    assert!(registry.jukebox_playing(PeerRegistry::JUKEBOX_PLAYING_WINDOW));
}
