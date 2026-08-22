use common::{Game, PlayerIdentity};

// A bare gamertag is the shape that made channel membership a silent no-op when it
// reached a keyed lookup. Parsing must refuse it rather than pass it through, which is
// the opposite of `Game::display_name`.
#[test]
fn a_bare_gamertag_is_not_an_identity() {
    assert!("Alaydriem".parse::<PlayerIdentity>().is_err());
}

#[test]
fn an_unknown_game_prefix_is_refused() {
    assert!("hytale:Alaydriem".parse::<PlayerIdentity>().is_err());
}

#[test]
fn an_empty_gamertag_is_refused() {
    assert!("minecraft:".parse::<PlayerIdentity>().is_err());
}

// Xbox gamertags contain spaces, and the gamertag may itself contain a colon. Splitting
// anywhere but the first colon would turn one player into two.
#[test]
fn only_the_first_colon_separates() {
    let id: PlayerIdentity = "minecraft:Some: Gamer".parse().expect("parses");
    assert_eq!(id.gamertag(), "Some: Gamer");
    assert_eq!(id.game(), &Game::Minecraft);
}

#[test]
fn display_round_trips_through_parse() {
    let id = PlayerIdentity::new(Game::Minecraft, "Some Gamer");
    assert_eq!(id.to_string(), "minecraft:Some Gamer");
    assert_eq!(id.to_string().parse::<PlayerIdentity>().unwrap(), id);
}

// A regression guard for a cross-language contract, not a serde test. The TypeScript
// bindings and every stored channel see a string; if the human-readable form ever
// becomes an object, `channel.creator === self` in GroupsView.ts silently evaluates
// false for every player and reads as "nobody owns any group" rather than as an error.
#[test]
fn the_human_readable_form_is_the_canonical_string() {
    let id = PlayerIdentity::new(Game::Minecraft, "Alaydriem");
    assert_eq!(
        serde_json::to_string(&id).unwrap(),
        "\"minecraft:Alaydriem\""
    );
    assert_eq!(
        serde_json::from_str::<PlayerIdentity>("\"minecraft:Alaydriem\"").unwrap(),
        id
    );
}

// The human-readable form must reject a bare gamertag too, or a hand-edited channel
// file reintroduces exactly the value the type exists to make unconstructible.
#[test]
fn the_human_readable_form_refuses_a_bare_gamertag() {
    assert!(serde_json::from_str::<PlayerIdentity>("\"Alaydriem\"").is_err());
}

// The binary form drops the game prefix text in favour of a discriminant. This is the
// saving that pays for the audio frame envelope, so it is asserted rather than assumed.
#[test]
fn the_binary_form_is_smaller_than_the_string_form() {
    let id = PlayerIdentity::new(Game::Minecraft, "Alaydriem");
    let typed = postcard::to_stdvec(&id).unwrap();
    let as_string = postcard::to_stdvec(&id.to_string()).unwrap();
    assert!(
        typed.len() < as_string.len(),
        "typed {} should be smaller than string {}",
        typed.len(),
        as_string.len()
    );
    assert_eq!(postcard::from_bytes::<PlayerIdentity>(&typed).unwrap(), id);
}
