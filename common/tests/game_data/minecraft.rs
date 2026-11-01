use common::game_data::{Dimension, DimensionHash};

#[test]
fn known_names_map_with_and_without_the_minecraft_prefix() {
    for (raw, want) in [
        ("overworld", Dimension::Overworld),
        ("minecraft:overworld", Dimension::Overworld),
        ("nether", Dimension::TheNether),
        ("minecraft:nether", Dimension::TheNether),
        ("minecraft:the_nether", Dimension::TheNether),
        ("the_end", Dimension::TheEnd),
        ("minecraft:the_end", Dimension::TheEnd),
        ("death", Dimension::Death),
    ] {
        assert_eq!(Dimension::from(raw), want, "{raw}");
    }
}

#[test]
fn an_unknown_name_is_other_never_overworld() {
    assert!(matches!(
        Dimension::from("twilightforest:twilight_forest"),
        Dimension::Other(_)
    ));
    assert!(matches!(Dimension::from("aether"), Dimension::Other(_)));
}

#[test]
fn only_one_leading_minecraft_prefix_is_removed() {
    assert_eq!(Dimension::from("minecraft:foo"), Dimension::from("foo"));
    assert_ne!(
        Dimension::from("minecraft:minecraft:foo"),
        Dimension::from("foo")
    );
}

#[test]
fn case_is_significant() {
    assert_ne!(Dimension::from("Foo"), Dimension::from("foo"));
}

// The hash is compared across server versions, relay peers and bridges, so the
// formula is a contract: this value must never change.
#[test]
fn the_hash_formula_is_locked() {
    assert_eq!(
        Dimension::from("twilightforest:twilight_forest").as_api_str(),
        "custom:2805eb64d4e16523"
    );
}

#[test]
fn the_custom_form_restores_the_hash_without_rehashing() {
    let dimension = Dimension::from("mypack:aether");
    assert_eq!(Dimension::from(dimension.as_api_str().as_ref()), dimension);
}

#[test]
fn a_malformed_custom_form_is_hashed_as_a_raw_name() {
    for raw in ["custom:aether", "custom:ABCDEF0123456789", "custom:0123"] {
        let dimension = Dimension::from(raw);
        assert_eq!(dimension, Dimension::Other(DimensionHash::of(raw)), "{raw}");
    }
}

#[test]
fn an_empty_name_is_isolated_not_overworld() {
    assert_eq!(Dimension::from(""), Dimension::Other(DimensionHash::of("")));
}

#[test]
fn json_carries_bare_strings() {
    assert_eq!(
        serde_json::to_string(&Dimension::TheNether).unwrap(),
        r#""nether""#
    );
    assert_eq!(
        serde_json::to_string(&Dimension::TheSift).unwrap(),
        r#""the_sift""#
    );

    let parsed: Dimension = serde_json::from_str(r#""mypack:aether""#).unwrap();
    assert_eq!(parsed, Dimension::from("mypack:aether"));
    assert_eq!(
        serde_json::to_value(parsed).unwrap(),
        serde_json::Value::String(parsed.as_api_str().into_owned())
    );
}

// Postcard encodes the variant index, so the reserved slot and the hashed variant
// must keep these positions for peers to agree.
#[test]
fn postcard_reserves_index_4_for_the_sift_and_5_for_other() {
    assert_eq!(postcard::to_stdvec(&Dimension::TheSift).unwrap(), vec![4]);

    let other = Dimension::from("mypack:aether");
    let encoded = postcard::to_stdvec(&other).unwrap();
    assert_eq!(encoded.len(), 9);
    assert_eq!(encoded[0], 5);
    assert_eq!(postcard::from_bytes::<Dimension>(&encoded).unwrap(), other);
}

// Frames and JSON carry the hash as `custom:<hex>`; errors and debug logs must print the
// same hex or an operator cannot match a mismatch to the dimension behind it.
#[test]
fn the_debug_form_shows_the_hex_the_wire_form_carries() {
    let dimension = Dimension::from("twilightforest:twilight_forest");

    assert!(format!("{dimension:?}").contains("2805eb64d4e16523"), "{dimension:?}");
}

#[test]
fn a_huge_name_costs_the_same_nine_bytes() {
    let huge = "x".repeat(1 << 20);
    assert_eq!(
        postcard::to_stdvec(&Dimension::from(huge.as_str())).unwrap().len(),
        9
    );
}
