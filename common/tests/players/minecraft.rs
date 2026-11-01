use common::MinecraftPlayer;
use common::errors::{CommunicationError, GameError, MinecraftCommunicationError};
use common::game_data::Dimension;

use super::fixture::PlayerFixture;

#[test]
fn world_uuid_mismatch_blocks_communication() {
    let a = PlayerFixture::make(Some("world-a"));
    let b = PlayerFixture::make(Some("world-b"));
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
    let a = PlayerFixture::make(Some("world-a"));
    let b = PlayerFixture::make(Some("world-a"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_none_none_allows_communication() {
    let a = PlayerFixture::make(None);
    let b = PlayerFixture::make(None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_some_none_allows_communication() {
    let a = PlayerFixture::make(Some("world-a"));
    let b = PlayerFixture::make(None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn world_uuid_none_some_allows_communication() {
    let a = PlayerFixture::make(None);
    let b = PlayerFixture::make(Some("world-a"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn relay_world_uuid_same_and_world_uuid_none_allows_communication() {
    let a = PlayerFixture::make_with_relay(None, Some("realm-1"));
    let b = PlayerFixture::make_with_relay(None, Some("realm-1"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn relay_world_uuid_different_blocks_communication() {
    let a = PlayerFixture::make_with_relay(None, Some("realm-1"));
    let b = PlayerFixture::make_with_relay(None, Some("realm-2"));
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
    let a = PlayerFixture::make_with_relay(Some("world-a"), None);
    let b = PlayerFixture::make_with_relay(Some("world-a"), None);
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

fn in_dimension(dimension: Dimension) -> MinecraftPlayer {
    MinecraftPlayer {
        dimension,
        ..PlayerFixture::make(None)
    }
}

fn is_dimension_mismatch(err: &CommunicationError) -> bool {
    matches!(
        err,
        CommunicationError::Game(GameError::Minecraft(
            MinecraftCommunicationError::DimensionMismatch { .. }
        ))
    )
}

#[test]
fn different_custom_dimensions_cannot_hear_each_other() {
    let a = in_dimension(Dimension::from("mypack:aether"));
    let b = in_dimension(Dimension::from("other:aether"));
    let err = a.can_communicate_with(&b, 100.0).unwrap_err();
    assert!(is_dimension_mismatch(&err), "{err:?}");
}

#[test]
fn the_same_custom_dimension_in_range_can() {
    let a = in_dimension(Dimension::from("mypack:aether"));
    let b = in_dimension(Dimension::from("mypack:aether"));
    assert!(a.can_communicate_with(&b, 100.0).is_ok());
}

#[test]
fn a_custom_dimension_cannot_hear_overworld() {
    let a = in_dimension(Dimension::from("mypack:aether"));
    let b = in_dimension(Dimension::Overworld);
    let err = a.can_communicate_with(&b, 100.0).unwrap_err();
    assert!(is_dimension_mismatch(&err), "{err:?}");
}

// Mods and servers are released on their own schedules. An old server rejects a player whose
// JSON lacks `deafen`, and an old mod still sends `deafen`, so the key is a cross-version
// contract even though the field is named for what it does.
#[test]
fn the_flag_keeps_its_wire_key() {
    let mut player = PlayerFixture::make(None);
    player.whispering = true;

    let json = serde_json::to_value(&player).expect("serialize");
    assert_eq!(json["deafen"], serde_json::Value::Bool(true), "{json}");
    assert!(json.get("whispering").is_none(), "{json}");

    let read: common::MinecraftPlayer = serde_json::from_value(json).expect("deserialize");
    assert!(read.whispering);
}
