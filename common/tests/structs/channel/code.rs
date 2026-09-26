use common::structs::channel::GroupCode;

#[test]
fn a_generated_code_is_two_groups_of_four_over_the_alphabet() {
    for _ in 0..200 {
        let code = GroupCode::generate();
        let (left, right) = code.split_once('-').expect("one dash");
        assert_eq!(left.len(), 4, "{code}");
        assert_eq!(right.len(), 4, "{code}");
        for c in code.chars().filter(|c| *c != '-') {
            assert!(
                "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c),
                "{code} carries {c}"
            );
        }
    }
}

#[test]
fn a_generated_code_normalizes_to_itself() {
    let code = GroupCode::generate();
    assert_eq!(GroupCode::normalize(&code).as_deref(), Some(code.as_str()));
}

#[test]
fn case_spacing_and_the_dash_are_all_optional() {
    let canonical = "AB12-CD34";
    for typed in ["ab12cd34", "AB12CD34", "ab12-cd34", "  AB12 CD34 ", "AB 12 CD 34"] {
        assert_eq!(
            GroupCode::normalize(typed).as_deref(),
            Some(canonical),
            "{typed}"
        );
    }
}

#[test]
fn the_characters_the_alphabet_omits_map_to_the_digits_they_are_misread_as() {
    // I, L and O are never generated, so a player who types one meant the digit.
    assert_eq!(
        GroupCode::normalize("IL0O-1234").as_deref(),
        Some("1100-1234")
    );
}

#[test]
fn a_code_of_the_wrong_length_is_refused() {
    assert_eq!(GroupCode::normalize("AB12-CD3"), None);
    assert_eq!(GroupCode::normalize("AB12-CD345"), None);
    assert_eq!(GroupCode::normalize(""), None);
}

#[test]
fn a_code_carrying_a_character_outside_the_alphabet_is_refused() {
    // U is excluded on purpose and has no digit to map to.
    assert_eq!(GroupCode::normalize("ABU2-CD34"), None);
    assert_eq!(GroupCode::normalize("AB12-CD3!"), None);
}

#[test]
fn a_legacy_nanoid_is_not_a_group_code() {
    // Codes issued before this format are 21 characters and must not half-parse.
    assert_eq!(GroupCode::normalize("V1StGXR8_Z5jdHi6B-myT"), None);
}
