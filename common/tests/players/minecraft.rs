use common::errors::{CommunicationError, GameError, MinecraftCommunicationError};

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
