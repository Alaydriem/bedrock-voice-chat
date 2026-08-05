use std::collections::HashSet;

use bvc_server_lib::services::WorldIndex;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::{Coordinate, Orientation, PlayerEnum};

const WORLD: &str = "8f14e45f-ea8f-4b62-9f2a-1c0d7e3b4a55";

/// The cell is the feed's scope, which for a 48-block voice range is 120.
const CELL: f32 = 120.0;

fn player(name: &str, x: f32, z: f32) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate { x, y: 64.0, z },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: Some(WORLD.to_string()),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
    })
}

fn index(players: Vec<PlayerEnum>, on_voice: &[&str]) -> WorldIndex {
    let voice: HashSet<String> = on_voice.iter().map(|n| (*n).to_string()).collect();
    WorldIndex::build(players, voice, CELL)
}

#[test]
fn an_observer_who_is_not_in_the_world_is_absent_rather_than_an_error() {
    let world = index(vec![player("Alice", 0.0, 0.0)], &[]);

    assert!(world.observer("Bob").is_none());
}

// The neighbour lookup exists to replace a walk of the whole world, so somebody far away
// must not be a candidate at all.
#[test]
fn a_distant_player_is_not_a_neighbour() {
    let alice = player("Alice", 0.0, 0.0);
    let world = index(vec![alice.clone(), player("Far", 10_000.0, 0.0)], &[]);

    let names: Vec<String> = world
        .neighbours(&alice)
        .iter()
        .map(|p| {
            use common::traits::player_data::PlayerData;
            p.get_name().to_string()
        })
        .collect();

    assert_eq!(names, vec!["Alice".to_string()]);
}

/// The reason the cell is scope-sized rather than voice-sized.
///
/// A player one cell away can still be inside scope of the observer, and the three-by-three
/// ring is what guarantees they are found. Sized to voice range instead, somebody at 100
/// blocks would appear or vanish depending only on where the grid lines happened to fall.
#[test]
fn a_player_in_the_next_cell_is_still_a_neighbour() {
    let alice = player("Alice", 110.0, 0.0);
    let across = player("Across", 130.0, 0.0);
    let world = index(vec![alice.clone(), across], &[]);

    assert_eq!(world.neighbours(&alice).len(), 2);
}

#[test]
fn a_negative_coordinate_lands_in_its_own_cell_rather_than_the_origin() {
    let alice = player("Alice", -130.0, -130.0);
    let origin = player("Origin", 10.0, 10.0);
    let world = index(vec![alice.clone(), origin], &[]);

    // Truncating towards zero would put both in cell (0, 0) and make every player near the
    // origin a neighbour of every player just across the axis from it.
    assert_eq!(world.neighbours(&alice).len(), 1);
}

#[test]
fn voice_connections_are_tracked_separately_from_the_world() {
    let world = index(
        vec![player("OnVoice", 0.0, 0.0), player("GameOnly", 5.0, 0.0)],
        &["OnVoice"],
    );

    assert!(world.is_on_voice("OnVoice"));
    assert!(!world.is_on_voice("GameOnly"));
}
