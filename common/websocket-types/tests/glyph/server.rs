use websocket_types::{MarkPalette, ServerGlyph};

// Fixed vectors, produced by running `client/src/radial/core/glyph/ServerGlyph.ts` itself.
//
// These are the only thing keeping the two implementations equal. Nothing at runtime can detect
// the Rust palette or the Rust hash drifting from the TypeScript ones, and a drift means a
// controller draws a different tile for a server than the desktop draws for the same name.
// `client/src/radial/tests/identity.test.ts` asserts the same values from the other side.
#[test]
fn derives_the_same_glyph_as_the_client() {
    let cases = [
        ("bvc.alaydriem.com", "#f9871d", 17u8, "1b57eb5"),
        ("voice.hearthhold.net", "#f67414", 18, "15faa31"),
        ("a", "#f8e434", 14, "0a212a4"),
        ("Ops", "#466cf3", 4, "1b013fb"),
    ];

    for (name, hue, hue_index, pattern) in cases {
        let glyph = ServerGlyph::of(name);
        assert_eq!(glyph.hue, hue, "hue for {name}");
        assert_eq!(glyph.hue_index, hue_index, "hue index for {name}");
        assert_eq!(glyph.pattern, pattern, "pattern for {name}");
    }
}

// Seven characters for every input, including one whose grid is nearly empty. A consumer
// slicing fixed offsets breaks the moment a shorter string arrives.
#[test]
fn pattern_is_always_seven_lowercase_hex_characters() {
    for name in [
        "",
        "a",
        "bvc.alaydriem.com",
        "a much longer world name than usual",
    ] {
        let glyph = ServerGlyph::of(name);
        assert_eq!(glyph.pattern.len(), 7, "pattern width for {name:?}");
        assert!(
            glyph
                .pattern
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "pattern for {name:?} must be lowercase hex, got {}",
            glyph.pattern
        );
    }
}

// The hue is one of the mark's own columns and never an arbitrary colour, which is what makes a
// derived glyph look like it belongs to this product rather than like a random swatch.
#[test]
fn hue_is_always_a_column_of_the_mark() {
    for name in ["a", "bvc.example.com", "voice.hearthhold.net"] {
        let glyph = ServerGlyph::of(name);
        assert!((glyph.hue_index as usize) < MarkPalette::COLS);
        assert_eq!(glyph.hue, MarkPalette::hue_at(glyph.hue_index as usize));
    }
}

// Two worlds a picker shows side by side have to be distinguishable, which is the whole reason
// the glyph travels at all. The hue alone does not achieve it: there are 23 of them.
#[test]
fn different_names_get_different_patterns() {
    let names = [
        "bvc.alaydriem.com",
        "voice.hearthhold.net",
        "bvc.tinyaxolotl.gg",
        "a",
        "b",
    ];

    let patterns: std::collections::HashSet<String> =
        names.iter().map(|n| ServerGlyph::of(n).pattern).collect();

    assert_eq!(patterns.len(), names.len());
}

// Non-ASCII is where an implementation hashing bytes rather than UTF-16 code units silently
// disagrees with the client, and a world name with an accent is ordinary. Asserting the
// client's own values is what catches it: any hash makes these differ from their ASCII
// spelling, so only the exact expected value proves the two agree.
#[test]
fn hashes_code_units_so_non_ascii_names_stay_in_step() {
    let cases = [
        ("café", "#466cf3", 4u8, "1bfc7e4"),
        ("日本", "#8238d8", 1, "1bfa884"),
    ];

    for (name, hue, hue_index, pattern) in cases {
        let glyph = ServerGlyph::of(name);
        assert_eq!(glyph.hue, hue, "hue for {name}");
        assert_eq!(glyph.hue_index, hue_index, "hue index for {name}");
        assert_eq!(glyph.pattern, pattern, "pattern for {name}");
    }
}
