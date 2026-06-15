use common::errors::{CommunicationError, GameError, MinecraftCommunicationError};
use common::game_data::Dimension;
use common::{Coordinate, MinecraftPlayer, Orientation};

fn make_player(world_uuid: Option<&str>) -> MinecraftPlayer {
    MinecraftPlayer {
        name: "Player".to_string(),
        coordinates: Coordinate { x: 0.0, y: 0.0, z: 0.0 },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: world_uuid.map(String::from),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
    }
}

fn make_player_with_relay(world_uuid: Option<&str>, relay_world_uuid: Option<&str>) -> MinecraftPlayer {
    MinecraftPlayer {
        relay_world_uuid: relay_world_uuid.map(String::from),
        ..make_player(world_uuid)
    }
}

#[test]
fn world_uuid_mismatch_blocks_communication() {
    let a = make_player(Some("world-a"));
    let b = make_player(Some("world-b"));
    let err = a.can_communicate_with(&b, 100.0).unwrap_err();
    assert!(matches!(
        err,
        CommunicationError::Game(GameError::Minecraft(
            MinecraftCommunicationError::WorldMismatch { .. }
        ))
    ));
}

#[test]
fn world_uuid_match_allows_communication() {
    let a = make_player(Some("world-a"));
    let b = make_player(Some("world-a"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_none_none_allows_communication() {
    let a = make_player(None);
    let b = make_player(None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_some_none_allows_communication() {
    let a = make_player(Some("world-a"));
    let b = make_player(None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_none_some_allows_communication() {
    let a = make_player(None);
    let b = make_player(Some("world-a"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn relay_world_uuid_same_and_world_uuid_none_allows_communication() {
    let a = make_player_with_relay(None, Some("realm-1"));
    let b = make_player_with_relay(None, Some("realm-1"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn relay_world_uuid_different_blocks_communication() {
    let a = make_player_with_relay(None, Some("realm-1"));
    let b = make_player_with_relay(None, Some("realm-2"));
    let err = a.can_communicate_with(&b, 100.0).unwrap_err();
    assert!(matches!(
        err,
        CommunicationError::Game(GameError::Minecraft(
            MinecraftCommunicationError::WorldMismatch { .. }
        ))
    ));
}

#[test]
fn relay_world_uuid_none_both_world_uuid_equal_allows_communication() {
    let a = make_player_with_relay(Some("world-a"), None);
    let b = make_player_with_relay(Some("world-a"), None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}
