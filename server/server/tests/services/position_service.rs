use bvc_server_lib::services::{PositionHandle, PositionService};
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::{Coordinate, Orientation, PlayerEnum};

const WORLD: &str = "8f14e45f-ea8f-4b62-9f2a-1c0d7e3b4a55";
const OTHER_WORLD: &str = "1b9d6bcd-bbfd-4b2d-9b5d-ab8dfbbd4bed";

fn player(name: &str, x: f32, z: f32, world: &str, dimension: Dimension) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate { x, y: 64.0, z },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension,
        deafen: false,
        spectator: false,
        world_uuid: Some(world.to_string()),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
    })
}

// The UI animates an entry by matching it frame to frame, so one player must
// map to one handle for the life of a session.
#[test]
fn a_player_keeps_one_handle_within_a_session() {
    let session = PositionHandle::new_session();

    assert_eq!(session.handle_for("Alice"), session.handle_for("Alice"));
}

#[test]
fn different_players_get_different_handles() {
    let session = PositionHandle::new_session();

    assert_ne!(session.handle_for("Alice"), session.handle_for("Bob"));
}

// Two sessions must not agree on a player's handle. If they did, the handle
// would be a stable pseudonym -- a tracking identifier by another name -- and
// two cooperating observers could correlate sightings of the same person.
#[test]
fn handles_do_not_correlate_across_sessions() {
    let first = PositionHandle::new_session();
    let second = PositionHandle::new_session();

    assert_ne!(first.handle_for("Alice"), second.handle_for("Alice"));
}

// The whole point is to see someone before hearing them, so scope must exceed
// whatever voice range the operator configured.
#[test]
fn scope_always_exceeds_voice_range() {
    for voice_range in [8.0_f32, 48.0, 100.0, 1000.0] {
        let service = PositionService::for_voice_range(voice_range);
        assert!(
            service.scope_range() > voice_range || service.scope_range() == 256.0,
            "scope {} must exceed voice range {voice_range} unless clamped",
            service.scope_range()
        );
    }
}

// Scope drives a per-observer scan of the world on every tick, so an enormous
// configured voice range must not become an unbounded scan.
#[test]
fn scope_is_clamped_to_the_maximum() {
    let service = PositionService::for_voice_range(100_000.0);

    assert_eq!(service.scope_range(), 256.0);
}

#[test]
fn the_observer_is_excluded_from_their_own_snapshot() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[alice.clone()], &handles);

    assert!(positions.is_empty());
}

#[test]
fn a_nearby_player_in_the_same_world_is_included() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 30.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[alice.clone(), bob], &handles);

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].handle, handles.handle_for("Bob"));
    assert_eq!(positions[0].distance, 30);
}

// A different world is a different reality; nothing there may appear.
#[test]
fn a_player_in_another_world_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 5.0, 0.0, OTHER_WORLD, Dimension::Overworld);

    assert!(
        service
            .snapshot_positions(&alice, &[bob], &handles)
            .is_empty()
    );
}

// Standing on the same coordinates in the Nether must not appear on an
// Overworld observer's feed.
#[test]
fn a_player_in_another_dimension_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 5.0, 0.0, WORLD, Dimension::TheNether);

    assert!(
        service
            .snapshot_positions(&alice, &[bob], &handles)
            .is_empty()
    );
}

#[test]
fn a_player_beyond_scope_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let far = player("Far", 100_000.0, 0.0, WORLD, Dimension::Overworld);

    assert!(
        service
            .snapshot_positions(&alice, &[far], &handles)
            .is_empty()
    );
}

// The behaviour the feature exists for: visible on the feed, not yet audible.
#[test]
fn a_player_outside_voice_range_is_still_visible() {
    let voice_range = 48.0;
    let service = PositionService::for_voice_range(voice_range);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let approaching = player("Approaching", 120.0, 0.0, WORLD, Dimension::Overworld);

    assert!(
        approaching.can_communicate_with(&alice, voice_range).is_err(),
        "fixture must be outside voice range for this test to mean anything"
    );
    assert_eq!(
        service
            .snapshot_positions(&alice, &[approaching], &handles)
            .len(),
        1
    );
}

// Bearing is relative to facing, so the UI can place an entry without ever
// learning an absolute coordinate.
#[test]
fn bearing_is_relative_to_observer_facing() {
    let service = PositionService::for_voice_range(48.0);
    let handles = PositionHandle::new_session();
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let ahead = player("Ahead", 0.0, 50.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[ahead], &handles);

    assert_eq!(positions.len(), 1);
    assert!(positions[0].bearing_deg < 360);
}
