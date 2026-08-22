use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::SpeakerPosition;
use common::{Coordinate, Orientation, PlayerEnum};

fn speaker() -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: "SomeGamertag".to_string(),
        coordinates: Coordinate {
            x: 128.5,
            y: 64.0,
            z: -1024.25,
        },
        orientation: Orientation { x: 12.5, y: -3.25 },
        dimension: Dimension::Overworld,
        deafen: true,
        spectator: false,
        world_uuid: Some("8f14e45f-ceea-467a-9575-1b0aaf2c1e6c".to_string()),
        alternative_identity: None,
        player_uuid: Some("c9f0f895-fb98-4b0f-b0a8-3f1c8e0f4a2d".to_string()),
        relay_world_uuid: None,
        bridged_voice: false,
    })
}

// The whole point of the type. If it is not materially smaller than the player it replaces,
// nothing has been gained — and this is the guard that a later field addition shows up as a
// number rather than as a slow regression.
#[test]
fn it_is_far_smaller_than_the_player_it_replaces() {
    let player = speaker();
    let full = postcard::to_stdvec(&Some(player.clone())).unwrap();
    let reduced = postcard::to_stdvec(&Some(SpeakerPosition::from_player(&player))).unwrap();

    assert!(
        reduced.len() * 4 < full.len(),
        "reduced {} should be far smaller than the player's {}",
        reduced.len(),
        full.len()
    );
}

// A deafened speaker plays centre-panned at unity rather than attenuated, so losing this flag
// is inaudible rather than obvious — which is exactly why it is asserted.
#[test]
fn a_deafened_speaker_stays_deafened() {
    let reduced = SpeakerPosition::from_player(&speaker());
    assert!(reduced.deafened);

    let round_tripped: SpeakerPosition =
        postcard::from_bytes(&postcard::to_stdvec(&reduced).unwrap()).unwrap();
    assert!(round_tripped.deafened);
}

// The position is the one field a listener pans from. Taking it from the player rather than
// from a caller is what stops the two disagreeing.
#[test]
fn the_position_comes_from_the_player() {
    let reduced = SpeakerPosition::from_player(&speaker());
    assert_eq!(reduced.position.x, 128.5);
    assert_eq!(reduced.position.y, 64.0);
    assert_eq!(reduced.position.z, -1024.25);
}
