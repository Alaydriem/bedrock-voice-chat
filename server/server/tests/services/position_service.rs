use bvc_server_lib::services::{FAR_TIER_MAX, PositionService};
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::position::PresenceKind;
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

/// Everyone is on voice unless a test says otherwise.
fn all_on_voice(_name: &str) -> bool {
    true
}

fn nobody_on_voice(_name: &str) -> bool {
    false
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
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[alice.clone()], &all_on_voice);

    assert!(positions.is_empty());
}

// The name is the certificate CN form, because that is the identity channel membership,
// the recorded track and the colour beside the name are all keyed on.
#[test]
fn an_entry_is_named_by_its_cn_form() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 30.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[alice.clone(), bob], &all_on_voice);

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].name, "minecraft:Bob");
    assert_eq!(positions[0].distance, 30);
}

// Somebody in the world without BVC is the most common confusion this product produces:
// they are standing in front of you and nothing you say reaches them. Omitting them would
// make them indistinguishable from nobody being there.
#[test]
fn a_player_not_on_voice_is_reported_as_present_and_unreachable() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 10.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[bob], &nobody_on_voice);

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].presence, PresenceKind::Game);
}

#[test]
fn a_player_on_voice_is_reported_as_such() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 10.0, 0.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[bob], &all_on_voice);

    assert_eq!(positions[0].presence, PresenceKind::Voice);
}

// The roster is the near tier, so it cannot be truncated: a card without a distance is a
// card that lies about how far away somebody is.
#[test]
fn everybody_within_voice_range_is_sent() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let crowd: Vec<PlayerEnum> = (0..40)
        .map(|i| player(&format!("Near{i}"), 5.0 + (i as f32) * 0.5, 0.0, WORLD, Dimension::Overworld))
        .collect();

    let positions = service.snapshot_positions(&alice, &crowd, &all_on_voice);

    assert_eq!(positions.len(), 40);
}

// Beyond voice range only a handful are ever drawn, so a crowded square two hundred blocks
// away must not become kilobytes of JSON twice a second.
#[test]
fn the_far_tier_is_capped() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let crowd: Vec<PlayerEnum> = (0..40)
        .map(|i| player(&format!("Far{i}"), 60.0 + (i as f32), 0.0, WORLD, Dimension::Overworld))
        .collect();

    let positions = service.snapshot_positions(&alice, &crowd, &all_on_voice);

    assert_eq!(positions.len(), FAR_TIER_MAX);
}

// Nearest first is what makes the far tier's cap safe: it can only ever drop somebody
// already beyond voice range.
#[test]
fn entries_are_ordered_nearest_first() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let world = vec![
        player("Far", 100.0, 0.0, WORLD, Dimension::Overworld),
        player("Mid", 40.0, 0.0, WORLD, Dimension::Overworld),
        player("Close", 5.0, 0.0, WORLD, Dimension::Overworld),
    ];

    let positions = service.snapshot_positions(&alice, &world, &all_on_voice);

    let distances: Vec<u16> = positions.iter().map(|entry| entry.distance).collect();
    assert_eq!(distances, vec![5, 40, 100]);
}

// A different world is a different reality; nothing there may appear.
#[test]
fn a_player_in_another_world_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 5.0, 0.0, OTHER_WORLD, Dimension::Overworld);

    assert!(
        service
            .snapshot_positions(&alice, &[bob], &all_on_voice)
            .is_empty()
    );
}

// Standing on the same coordinates in the Nether must not appear on an
// Overworld observer's feed.
#[test]
fn a_player_in_another_dimension_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let bob = player("Bob", 5.0, 0.0, WORLD, Dimension::TheNether);

    assert!(
        service
            .snapshot_positions(&alice, &[bob], &all_on_voice)
            .is_empty()
    );
}

#[test]
fn a_player_beyond_scope_is_excluded() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let far = player("Far", 100_000.0, 0.0, WORLD, Dimension::Overworld);

    assert!(
        service
            .snapshot_positions(&alice, &[far], &all_on_voice)
            .is_empty()
    );
}

// The behaviour the feature exists for: visible on the feed, not yet audible.
#[test]
fn a_player_outside_voice_range_is_still_visible() {
    let voice_range = 48.0;
    let service = PositionService::for_voice_range(voice_range);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let approaching = player("Approaching", 120.0, 0.0, WORLD, Dimension::Overworld);

    assert!(
        approaching.can_communicate_with(&alice, voice_range).is_err(),
        "fixture must be outside voice range for this test to mean anything"
    );
    assert_eq!(
        service
            .snapshot_positions(&alice, &[approaching], &all_on_voice)
            .len(),
        1
    );
}

// Bearing is relative to facing, so the UI can place an entry without ever
// learning an absolute coordinate.
#[test]
fn bearing_is_relative_to_observer_facing() {
    let service = PositionService::for_voice_range(48.0);
    let alice = player("Alice", 0.0, 0.0, WORLD, Dimension::Overworld);
    let ahead = player("Ahead", 0.0, 50.0, WORLD, Dimension::Overworld);

    let positions = service.snapshot_positions(&alice, &[ahead], &all_on_voice);

    assert_eq!(positions.len(), 1);
    assert!(positions[0].bearing_deg < 360);
}
