use common::Game;
use common::structs::control::{ClientAction, ClientActionType};

fn action(id: &str, game: Option<Game>) -> ClientAction {
    ClientAction {
        id: id.to_string(),
        game,
        action: ClientActionType::CreateGroup,
    }
}

// A group action keys channel membership on the actor's canonical identity. Deriving that
// from a hardcoded game rather than the declared one puts a player's membership under the
// wrong key, which the audio router then never finds.
#[test]
fn the_actor_key_uses_the_declared_game() {
    assert_eq!(
        action("Alaydriem", Some(Game::Minecraft)).actor_key(),
        "minecraft:Alaydriem"
    );
    assert_eq!(action("Alaydriem", None).actor_key(), "minecraft:Alaydriem");
}

// The BDS and Java encoders do not consume `common`, so a mod that predates the field keeps
// working — on the Minecraft path, which is what it was implicitly using before the field
// existed.
#[test]
fn a_missing_game_falls_back_to_minecraft() {
    assert_eq!(action("Alaydriem", None).actor_key(), "minecraft:Alaydriem");
}

// Xbox gamertags contain spaces, and the key is built by concatenation rather than by
// splitting, so a two-word name stays one identity.
#[test]
fn a_name_with_spaces_stays_whole() {
    assert_eq!(
        action("Some Gamer", Some(Game::Minecraft)).actor_key(),
        "minecraft:Some Gamer"
    );
}

// `game` is additive on a wire shape two other languages encode by hand. Omitting it must
// deserialize rather than fail, or every un-updated mod breaks at once.
#[test]
fn an_action_without_a_game_still_deserializes() {
    let json = r#"{"id":"Alaydriem","action":"CreateGroup"}"#;
    let parsed: ClientAction = serde_json::from_str(json).expect("deserializes");
    assert_eq!(parsed.game, None);
    assert_eq!(parsed.actor_key(), "minecraft:Alaydriem");
}
