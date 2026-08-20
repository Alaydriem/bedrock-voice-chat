use common::traits::player_data::PlayerData;
use common::{Coordinate, Game, GenericPlayer, Orientation, PlayerEnum};

fn generic() -> PlayerEnum {
    PlayerEnum::Generic(GenericPlayer {
        name: "Alice".to_string(),
        coordinates: Coordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        game: Game::Minecraft,
    })
}

// Generic sits at postcard index 2 and must stay there. A build that dropped the slot at
// index 1 would encode Generic as 1, and every peer still on the old encoding would decode
// one player's voice frame as another player's.
#[test]
fn generic_still_encodes_at_its_original_index() {
    let bytes = postcard::to_stdvec(&generic()).expect("encode");

    assert_eq!(
        bytes.first().copied(),
        Some(0x02),
        "Generic must encode at index 2: {bytes:?}"
    );
}

// A datagram from a build that still sends a player at index 1 has to land somewhere inert.
// Decoding it as Generic would put a player carrying no data into the world.
#[test]
fn a_player_at_the_reserved_index_decodes_to_the_reserved_slot() {
    let decoded: PlayerEnum = postcard::from_bytes(&[0x01]).expect("decode");

    assert!(decoded.is_reserved());
}

// Nothing reads these in practice, because both ingestion paths drop Reserved first. They
// must still answer rather than panic: the decode happens on the QUIC hot path.
#[test]
fn the_reserved_slot_answers_inertly_rather_than_panicking() {
    let reserved = PlayerEnum::Reserved;

    assert_eq!(reserved.get_name(), "");
    assert_eq!(reserved.world_identifier(), None);
    assert_eq!(reserved.dimension(), None);
    assert!(!reserved.has_bridged_voice());
    assert!(reserved.is_deafened());
}

#[test]
fn the_reserved_slot_can_communicate_with_nobody() {
    assert!(
        PlayerEnum::Reserved
            .can_communicate_with(&generic(), 1000.0)
            .is_err()
    );
}
