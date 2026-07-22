use bvc_client_lib::control::ControlActionsManager;

// Preferences key on exact gamertags everywhere downstream (the sink's
// name→client-id remap, the dashboard's player cards), so a typed control
// target must resolve onto the canonical key — a raw pass-through of a case or
// game-prefix variant forks a ghost entry that plays no audio and renders on
// no card.

#[test]
fn exact_match_passes_through_untouched() {
    assert_eq!(
        ControlActionsManager::canonicalize_target("Alice", &["Alice", "alice"]),
        "Alice"
    );
}

#[test]
fn case_variant_resolves_to_the_tracked_casing() {
    assert_eq!(
        ControlActionsManager::canonicalize_target("alaydriem", &["Alaydriem"]),
        "Alaydriem"
    );
}

#[test]
fn game_prefix_variants_resolve_in_both_directions() {
    assert_eq!(
        ControlActionsManager::canonicalize_target("minecraft:alice", &["Alice"]),
        "Alice"
    );
    assert_eq!(
        ControlActionsManager::canonicalize_target("alice", &["minecraft:Alice"]),
        "minecraft:Alice"
    );
}

#[test]
fn earlier_candidates_win_so_tracked_names_beat_store_keys() {
    // Callers order candidates by authority: tracked voice names first, then
    // existing store keys.
    assert_eq!(
        ControlActionsManager::canonicalize_target("alice", &["Alice", "ALICE"]),
        "Alice"
    );
}

#[test]
fn unknown_target_parks_unchanged() {
    assert_eq!(
        ControlActionsManager::canonicalize_target("Ghost", &["Alice", "Bob"]),
        "Ghost"
    );
}
