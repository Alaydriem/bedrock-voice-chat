use common::game_data::BoundedLogText;

// A dropped record's error text embeds the offending value, which a mod can make as large
// as the request body. It is both a cache key and a log field, so it must be capped.
#[test]
fn a_huge_text_is_capped() {
    let huge = "x".repeat(1 << 20);

    assert_eq!(
        BoundedLogText::of(&huge).chars().count(),
        BoundedLogText::MAX_CHARS
    );
}

#[test]
fn the_cap_never_splits_a_character() {
    let wide = "é".repeat(BoundedLogText::MAX_CHARS + 10);

    let bounded = BoundedLogText::of(&wide);

    assert_eq!(bounded, "é".repeat(BoundedLogText::MAX_CHARS));
}

#[test]
fn a_short_text_is_unchanged() {
    assert_eq!(BoundedLogText::of("invalid type: null"), "invalid type: null");
}
