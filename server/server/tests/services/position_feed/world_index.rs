use std::collections::HashSet;

use bvc_server_lib::services::WorldIndex;
use common::game_data::Dimension;
use common::players::{HytalePlayer, MinecraftPlayer};
use common::traits::player_data::PlayerData;
use common::{Coordinate, HytaleDimension, Orientation, PlayerEnum};

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
        bridged_voice: false,
    })
}

fn bridged(name: &str, x: f32, z: f32) -> PlayerEnum {
    let PlayerEnum::Minecraft(mut mc) = player(name, x, z) else {
        unreachable!("player() builds a Minecraft player");
    };
    mc.bridged_voice = true;
    PlayerEnum::Minecraft(mc)
}

fn hytale(name: &str, x: f32, z: f32) -> PlayerEnum {
    PlayerEnum::Hytale(HytalePlayer {
        name: name.to_string(),
        coordinates: Coordinate { x, y: 64.0, z },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: HytaleDimension::default(),
        deafen: false,
        spectator: false,
        world_uuid: Some(WORLD.to_string()),
        player_uuid: None,
    })
}

fn index(players: Vec<PlayerEnum>, on_voice: &[&str]) -> WorldIndex {
    let voice: HashSet<String> = on_voice.iter().map(|n| (*n).to_string()).collect();
    WorldIndex::build(players, voice, CELL)
}

#[test]
fn an_observer_who_is_not_in_the_world_is_absent_rather_than_an_error() {
    let world = index(vec![player("Alice", 0.0, 0.0)], &[]);

    assert!(world.observer("minecraft:Bob").is_none());
}

/// The index answers on the canonical identity and nothing else.
///
/// A bare gamertag missing is indistinguishable from being signed in but not yet in the game,
/// so the socket would send empty frames forever rather than report anything wrong.
#[test]
fn a_bare_gamertag_finds_nobody() {
    let world = index(vec![player("Alice", 0.0, 0.0)], &[]);

    assert!(world.observer("Alice").is_none());
    assert!(world.observer("minecraft:Alice").is_some());
}

/// One server can host both games at once, and then a shared gamertag is two people.
///
/// Keyed on the bare name, whichever of them the world reported second replaced the first, and
/// the loser's socket was served the winner's neighbours — somebody else's view of the world.
#[test]
fn the_same_gamertag_in_two_games_is_two_observers() {
    let world = index(
        vec![player("Alaydriem", 0.0, 0.0), hytale("Alaydriem", 5_000.0, 0.0)],
        &[],
    );

    assert_eq!(world.len(), 2);
    let mc = world.observer("minecraft:Alaydriem").expect("minecraft");
    let ht = world.observer("hytale:Alaydriem").expect("hytale");
    assert_eq!(mc.get_position().x, 0.0);
    assert_eq!(ht.get_position().x, 5_000.0);
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
    // The set comes from the connection registry, which names connections `game:gamertag`.
    let world = index(
        vec![player("OnVoice", 0.0, 0.0), player("GameOnly", 5.0, 0.0)],
        &["minecraft:OnVoice"],
    );

    assert!(world.is_on_voice("minecraft:OnVoice"));
    assert!(!world.is_on_voice("minecraft:GameOnly"));
}

/// A player whose voice connection belongs to a mod is on voice.
///
/// The connection registry counts only what this server terminates, so a Simple Voice Chat
/// player is absent from it however audible they are. Reported as game-only, the client
/// hides their volume control and tells everyone beside them that they cannot hear you —
/// while their audio is arriving over the peer link.
#[test]
fn a_bridged_player_is_on_voice_without_a_connection_of_our_own() {
    let world = index(
        vec![bridged("Bridged", 0.0, 0.0), player("GameOnly", 5.0, 0.0)],
        &[],
    );

    assert!(world.is_on_voice("minecraft:Bridged"));
    assert!(!world.is_on_voice("minecraft:GameOnly"));
}

/// Bridged voice is added to the registry's set, never substituted for it.
///
/// Both populations share one server, so a set built from either source alone silently
/// demotes the other's players to game-only.
#[test]
fn bridged_voice_does_not_displace_this_servers_own_connections() {
    let world = index(
        vec![bridged("Bridged", 0.0, 0.0), player("OnQuic", 5.0, 0.0)],
        &["minecraft:OnQuic"],
    );

    assert!(world.is_on_voice("minecraft:Bridged"));
    assert!(world.is_on_voice("minecraft:OnQuic"));
}

/// Keyed the same way the registry keys its own, or the two sources disagree about one
/// person: a bare gamertag added here answers no to every lookup the feed makes.
#[test]
fn a_bridged_player_is_keyed_on_the_canonical_identity() {
    let world = index(vec![bridged("Bridged", 0.0, 0.0)], &[]);

    assert!(!world.is_on_voice("Bridged"));
    assert!(world.is_on_voice("minecraft:Bridged"));
}
