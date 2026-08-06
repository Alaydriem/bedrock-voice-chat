use common::traits::player_data::PlayerData;
use common::{MinecraftPlayer, PlayerEnum};

use crate::players::fixture::PlayerFixture;

fn minecraft(name: &str) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        ..PlayerFixture::make(None)
    })
}

// The identity a player is keyed on comes from the variant's own game, not from a caller's
// guess and not from a prefix stored in the name field.
#[test]
fn identity_is_the_variant_game_and_the_bare_name() {
    assert_eq!(minecraft("Alaydriem").identity(), "minecraft:Alaydriem");
}

// Xbox gamertags contain spaces. An identity that split on whitespace would turn one player
// into two.
#[test]
fn a_name_with_spaces_stays_whole() {
    assert_eq!(minecraft("Some Gamer").identity(), "minecraft:Some Gamer");
}

// The bare name is the display label and must survive unchanged, because the database, the
// gamerpic lookup and the alias table all key on it.
#[test]
fn get_name_stays_bare() {
    let player = minecraft("Alaydriem");
    assert_eq!(player.get_name(), "Alaydriem");
    assert_eq!(player.identity(), "minecraft:Alaydriem");
}
