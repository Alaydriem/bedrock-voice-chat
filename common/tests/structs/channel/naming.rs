use common::structs::channel::GroupName;

// A pinned index source. `next_with` calls it twice per attempt — adjective, then noun —
// so indices are supplied in pairs and the last value repeats once the script runs out.
fn pinned(script: Vec<usize>) -> impl FnMut(usize) -> usize {
    let mut remaining = script.into_iter();
    let mut last = 0;
    move |_bound| {
        if let Some(next) = remaining.next() {
            last = next;
        }
        last
    }
}

#[test]
fn draws_an_adjective_and_a_noun() {
    let name = GroupName::next(&[]);
    let (adjective, noun) = name.split_once(' ').expect("two words");
    assert!(
        GroupName::ADJECTIVES.contains(&adjective),
        "adjective: {adjective}"
    );
    assert!(GroupName::NOUNS.contains(&noun), "noun: {noun}");
}

#[test]
fn redraws_past_a_taken_name() {
    let first = format!("{} {}", GroupName::ADJECTIVES[0], GroupName::NOUNS[0]);
    let second = format!("{} {}", GroupName::ADJECTIVES[1], GroupName::NOUNS[1]);
    let mut rand = pinned(vec![0, 0, 1, 1]);
    assert_eq!(GroupName::next_with(&[first], &mut rand), second);
}

#[test]
fn numbers_the_name_when_every_draw_collides() {
    let first = format!("{} {}", GroupName::ADJECTIVES[0], GroupName::NOUNS[0]);
    let mut rand = pinned(vec![0]);
    assert_eq!(
        GroupName::next_with(&[first.clone()], &mut rand),
        format!("{first} 2")
    );
}

#[test]
fn numbering_continues_past_an_existing_ordinal() {
    let first = format!("{} {}", GroupName::ADJECTIVES[0], GroupName::NOUNS[0]);
    let taken = vec![first.clone(), format!("{first} 2")];
    let mut rand = pinned(vec![0]);
    assert_eq!(GroupName::next_with(&taken, &mut rand), format!("{first} 3"));
}

#[test]
fn the_taken_set_ignores_case_and_surrounding_space() {
    let first = format!("{} {}", GroupName::ADJECTIVES[0], GroupName::NOUNS[0]);
    let taken = vec![format!("  {}  ", first.to_uppercase())];
    let mut rand = pinned(vec![0]);
    assert_eq!(GroupName::next_with(&taken, &mut rand), format!("{first} 2"));
}

#[test]
fn the_word_lists_carry_no_duplicates_and_no_wire_unsafe_characters() {
    for list in [GroupName::ADJECTIVES, GroupName::NOUNS] {
        let mut seen = std::collections::HashSet::new();
        for word in list {
            assert!(seen.insert(*word), "duplicate word: {word}");
            assert!(
                !word.contains([';', '=', ':', ' ']),
                "word needs escaping: {word}"
            );
        }
    }
}
