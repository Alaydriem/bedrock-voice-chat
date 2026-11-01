use bvc_relay_sdk::BvcDimensionCodec;
use common::game_data::Dimension;

#[test]
fn the_codec_agrees_with_common() {
    let codec = BvcDimensionCodec::new();
    for raw in ["minecraft:the_nether", "mypack:aether", ""] {
        assert_eq!(
            codec.wire_form(raw.to_string()),
            Dimension::from(raw).as_api_str(),
            "{raw}"
        );
    }
}

// The same literal as common's locked hash test: the bridge and the server must agree.
#[test]
fn the_codec_returns_the_locked_hash() {
    assert_eq!(
        BvcDimensionCodec::new().wire_form("twilightforest:twilight_forest".to_string()),
        "custom:2805eb64d4e16523"
    );
}
