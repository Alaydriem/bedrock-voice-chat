use bvc_client_lib::control::ControlActionsManager;
use common::Game;

// A control target arrives from a game mod as a bare in-game name, while every key it has to
// land on — the persisted gain store, the mixer's gain projection, the dashboard's cards — is
// the canonical `game:gamertag`. These pin the composition and the one looseness that survives
// it, because a target that resolves to the wrong key forks a ghost entry that plays no audio
// and renders on no card.

fn resolve(target: &str, candidates: &[&str]) -> String {
    ControlActionsManager::canonicalize_target(target, &Game::Minecraft, candidates)
}

#[test]
fn a_bare_target_is_composed_against_the_actions_game() {
    assert_eq!(resolve("Alice", &["minecraft:Alice"]), "minecraft:Alice");
}

#[test]
fn an_already_canonical_target_is_not_prefixed_twice() {
    assert_eq!(
        resolve("minecraft:Alice", &["minecraft:Alice"]),
        "minecraft:Alice"
    );
}

#[test]
fn case_variance_in_the_gamertag_resolves_to_the_tracked_casing() {
    // What a mod reports genuinely varies in case from what the certificate carried, so this
    // is the one comparison that stays loose.
    assert_eq!(
        resolve("alaydriem", &["minecraft:Alaydriem"]),
        "minecraft:Alaydriem"
    );
}

#[test]
fn the_game_prefix_must_match_exactly() {
    // `minecraft:Bob` and `hytale:Bob` are two people. A prefix-insensitive match would let a
    // control action from one game mute somebody in the other, which is the collision the
    // canonical key exists to prevent.
    assert_eq!(
        ControlActionsManager::canonicalize_target("Bob", &Game::Minecraft, &["hytale:Bob"]),
        "minecraft:Bob",
        "a hytale candidate must not answer for a minecraft target"
    );
}

#[test]
fn earlier_candidates_win_so_tracked_names_beat_store_keys() {
    // Callers order candidates by authority: tracked voice names first, then existing store
    // keys.
    assert_eq!(
        resolve("alice", &["minecraft:Alice", "minecraft:ALICE"]),
        "minecraft:Alice"
    );
}

#[test]
fn an_unknown_target_parks_under_its_canonical_form() {
    // Not under the raw name. A bare key is one nothing downstream looks up, so the entry
    // would be written and then never resolve; the canonical form resolves as soon as that
    // player is tracked.
    assert_eq!(resolve("Ghost", &["minecraft:Alice"]), "minecraft:Ghost");
}

#[test]
fn a_hytale_action_composes_a_hytale_key() {
    assert_eq!(
        ControlActionsManager::canonicalize_target("Carol", &Game::Hytale, &["hytale:Carol"]),
        "hytale:Carol"
    );
}

// The reserved jukebox target is not a player and must survive composition untouched. Composed
// against the game it would become `minecraft:#jukebox` and park as a ghost store entry that
// plays no audio and renders on no card — the exact failure the canonical key exists to prevent,
// arriving through the one target that is not a name.
#[test]
fn the_reserved_jukebox_target_is_never_composed_against_a_game() {
    assert_eq!(
        resolve(common::consts::audio::JUKEBOX_CONTROL_TARGET, &[]),
        common::consts::audio::JUKEBOX_CONTROL_TARGET
    );
}

// Even with players tracked, the sentinel must not be matched loosely onto one of them.
#[test]
fn the_reserved_jukebox_target_never_resolves_onto_a_player() {
    assert_eq!(
        resolve(
            common::consts::audio::JUKEBOX_CONTROL_TARGET,
            &["minecraft:Alice", "minecraft:Bob"]
        ),
        common::consts::audio::JUKEBOX_CONTROL_TARGET
    );
}
