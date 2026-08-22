use bvc_client_lib::audio::stream::stream_manager::output::SpeakerStateCache;
use common::players::GenericPlayer;
use common::{Coordinate, Game, Orientation, PlayerEnum};

fn player_at(x: f32) -> PlayerEnum {
    PlayerEnum::Generic(GenericPlayer {
        name: "tester".to_string(),
        coordinates: Coordinate { x, y: 0.0, z: 0.0 },
        orientation: Orientation { x: 0.0, y: 0.0 },
        game: Game::Minecraft,
    })
}

// A heartbeat frame names the speaker and carries their position. Every frame after it
// carries neither, so both have to survive here or a reduced frame has no speaker.
#[test]
fn a_heartbeat_teaches_the_cache_both_halves() {
    let cache = SpeakerStateCache::new();
    cache.resolve(
        "7",
        Some("minecraft:Alice".to_string()),
        Some(player_at(1.0)),
    );

    let recalled = cache.resolve("7", None, None).expect("state retained");
    assert_eq!(recalled.name, "minecraft:Alice");
    assert!(recalled.player.is_some());
}

#[test]
fn a_later_heartbeat_replaces_the_position() {
    let cache = SpeakerStateCache::new();
    cache.resolve("7", Some("minecraft:Bob".to_string()), Some(player_at(5.0)));
    cache.resolve("7", Some("minecraft:Bob".to_string()), Some(player_at(9.0)));

    let recalled = cache.resolve("7", None, None).expect("state retained");
    match recalled.player.expect("position retained") {
        PlayerEnum::Generic(p) => assert_eq!(p.coordinates.x, 9.0),
        _ => panic!("wrong variant"),
    }
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
        Some(player_at(1.0)),
    );
    cache.resolve(
        "2",
        Some("minecraft:Alaydriem".to_string()),
        Some(player_at(50.0)),
    );

    match cache
        .resolve("1", None, None)
        .expect("device 1")
        .player
        .expect("position")
    {
        PlayerEnum::Generic(p) => assert_eq!(p.coordinates.x, 1.0),
        _ => panic!("wrong variant"),
    }
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
        Some(player_at(12.0)),
    );

    let between = cache
        .resolve("jukebox-abc", Some("jukebox-abc".to_string()), None)
        .expect("named on every frame");
    assert_eq!(between.name, "jukebox-abc");
    match between.player.expect("position recovered from the cache") {
        PlayerEnum::Generic(p) => assert_eq!(p.coordinates.x, 12.0),
        _ => panic!("wrong variant"),
    }
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
    assert!(state.player.is_none());
}
