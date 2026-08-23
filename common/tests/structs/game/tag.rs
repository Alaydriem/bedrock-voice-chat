use common::Game;

// The tag parser is what stands between a certificate CN and the identity everything is
// keyed on. A tag it no longer knows must be refused, not coerced to the only game left:
// coercion would let a certificate issued for a different game resolve to a Minecraft
// identity and inherit that player's channel membership.
#[test]
fn a_retired_game_tag_is_refused_rather_than_coerced() {
    assert_eq!(Game::from_tag("hytale"), None);
}

#[test]
fn an_unknown_game_tag_is_refused() {
    assert_eq!(Game::from_tag("notagame"), None);
    assert_eq!(Game::from_tag(""), None);
}

#[test]
fn the_known_tag_still_parses() {
    assert_eq!(Game::from_tag("minecraft"), Some(Game::Minecraft));
}

// display_name returns a value nothing may key on, so it must leave a string with an
// unrecognised prefix intact rather than truncating it at some other colon.
#[test]
fn an_unrecognised_prefix_is_left_intact_for_display() {
    assert_eq!(Game::display_name("hytale:Alex"), "hytale:Alex");
    assert_eq!(Game::display_name("minecraft:Alex"), "Alex");
}
