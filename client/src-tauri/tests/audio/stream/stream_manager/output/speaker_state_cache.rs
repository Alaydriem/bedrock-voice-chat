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

#[test]
fn attached_state_is_returned_and_remembered() {
    let cache = SpeakerStateCache::new();
    let resolved = cache.resolve("minecraft:Alice", Some(player_at(1.0)));
    assert!(resolved.is_some());
    let recalled = cache.resolve("minecraft:Alice", None);
    assert!(recalled.is_some());
}

#[test]
fn frame_without_state_falls_back_to_last_attached() {
    let cache = SpeakerStateCache::new();
    cache.resolve("minecraft:Bob", Some(player_at(5.0)));
    cache.resolve("minecraft:Bob", Some(player_at(9.0)));
    let recalled = cache.resolve("minecraft:Bob", None).expect("state retained");
    match recalled {
        PlayerEnum::Generic(p) => assert_eq!(p.coordinates.x, 9.0),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn unknown_speaker_without_state_resolves_to_none() {
    let cache = SpeakerStateCache::new();
    assert!(cache.resolve("minecraft:Nobody", None).is_none());
}
