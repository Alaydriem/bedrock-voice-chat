use bvc_client_lib::analytics::PlayerIdentity;

#[test]
fn a_long_gamertag_keeps_three_leading_and_two_trailing_characters() {
    let id = PlayerIdentity::from_gamertag("Charlie123");
    assert_eq!(id.display, "Cha*****23");
}

#[test]
fn a_seven_character_gamertag_censors_the_middle_two() {
    let id = PlayerIdentity::from_gamertag("Charles");
    assert_eq!(id.display, "Cha**es");
}

#[test]
fn a_six_character_gamertag_censors_a_single_character() {
    let id = PlayerIdentity::from_gamertag("Charle");
    assert_eq!(id.display, "Cha*le");
}

#[test]
fn a_gamertag_under_six_characters_is_censored_entirely() {
    let id = PlayerIdentity::from_gamertag("Bob");
    assert_eq!(id.display, "***");
}

#[test]
fn an_empty_gamertag_censors_to_empty() {
    let id = PlayerIdentity::from_gamertag("");
    assert_eq!(id.display, "");
}

#[test]
fn the_hash_is_stable_and_sixteen_hex_characters() {
    let a = PlayerIdentity::from_gamertag("Charles");
    let b = PlayerIdentity::from_gamertag("Charles");
    assert_eq!(a.hash, b.hash);
    assert_eq!(a.hash.len(), 16);
}

#[test]
fn the_hash_ignores_gamertag_case() {
    let a = PlayerIdentity::from_gamertag("Charles");
    let b = PlayerIdentity::from_gamertag("CHARLES");
    assert_eq!(a.hash, b.hash);
}

#[test]
fn different_gamertags_hash_differently() {
    let a = PlayerIdentity::from_gamertag("Charles");
    let b = PlayerIdentity::from_gamertag("Charlie");
    assert_ne!(a.hash, b.hash);
}
