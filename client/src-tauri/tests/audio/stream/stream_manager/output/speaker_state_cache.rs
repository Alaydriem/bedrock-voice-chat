use bvc_client_lib::audio::stream::stream_manager::output::SpeakerStateCache;
use common::Coordinate;
use common::structs::packet::SpeakerPosition;

fn speaker_at(x: f32) -> SpeakerPosition {
    SpeakerPosition::new(Coordinate { x, y: 0.0, z: 0.0 }, false)
}

// A heartbeat frame names the speaker and carries their position. Every frame after it
// carries neither, so both have to survive here or a reduced frame has no speaker.
#[test]
fn a_heartbeat_teaches_the_cache_both_halves() {
    let cache = SpeakerStateCache::new();
    cache.resolve(
        "7",
        Some("minecraft:Alice".to_string()),
        Some(speaker_at(1.0)),
    );

    let recalled = cache.resolve("7", None, None).expect("state retained");
    assert_eq!(recalled.name, "minecraft:Alice");
    assert!(recalled.speaker.is_some());
}

#[test]
fn a_later_heartbeat_replaces_the_position() {
    let cache = SpeakerStateCache::new();
    cache.resolve("7", Some("minecraft:Bob".to_string()), Some(speaker_at(5.0)));
    cache.resolve("7", Some("minecraft:Bob".to_string()), Some(speaker_at(9.0)));

    let recalled = cache.resolve("7", None, None).expect("state retained");
    assert_eq!(recalled.speaker.expect("position retained").position.x, 9.0);
}

// A reduced frame for a device nothing has ever named cannot be attributed to a speaker.
#[test]
fn a_key_nothing_has_named_resolves_to_nothing() {
    let cache = SpeakerStateCache::new();
    assert!(cache.resolve("99", None, None).is_none());
}

// Two devices of one player are separate speakers. Sharing a cache slot would pan one
// from the other's position.
#[test]
fn two_devices_of_one_player_hold_separate_state() {
    let cache = SpeakerStateCache::new();
    cache.resolve(
        "1",
        Some("minecraft:Alaydriem".to_string()),
        Some(speaker_at(1.0)),
    );
    cache.resolve(
        "2",
        Some("minecraft:Alaydriem".to_string()),
        Some(speaker_at(50.0)),
    );

    assert_eq!(
        cache
            .resolve("1", None, None)
            .expect("device 1")
            .speaker
            .expect("position")
            .position
            .x,
        1.0
    );
}

// Injected audio names itself on every frame but only carries a position on the heartbeat.
// Without the cached position the client cannot attenuate it, and two concurrent jukeboxes
// bleed into each other.
#[test]
fn a_named_frame_without_a_position_recovers_the_cached_one() {
    let cache = SpeakerStateCache::new();
    cache.resolve(
        "jukebox-abc",
        Some("jukebox-abc".to_string()),
        Some(speaker_at(12.0)),
    );

    let between = cache
        .resolve("jukebox-abc", Some("jukebox-abc".to_string()), None)
        .expect("named on every frame");
    assert_eq!(between.name, "jukebox-abc");
    assert_eq!(
        between
            .speaker
            .expect("position recovered from the cache")
            .position
            .x,
        12.0
    );
}

// A speaker named before any position has arrived is still attributable, so presence and
// gain work while spatial panning waits for the first heartbeat that carries one.
#[test]
fn a_speaker_named_before_any_position_still_resolves() {
    let cache = SpeakerStateCache::new();
    let state = cache
        .resolve("4", Some("minecraft:Carol".to_string()), None)
        .expect("named");
    assert_eq!(state.name, "minecraft:Carol");
    assert!(state.speaker.is_none());
}
