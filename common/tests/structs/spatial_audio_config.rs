use common::structs::SpatialAudioConfig;

// The client's whisper curve ends at this edge and the server stops sending at it. Both read
// this one function, and dropping the factor would put the curve's silence inside the range
// the server still delivers, which is the hard edge this exists to remove.
#[test]
fn the_whisper_edge_includes_the_proximity_factor() {
    let config = SpatialAudioConfig {
        whisper_distance: 4.0,
        ..SpatialAudioConfig::default()
    };

    assert_eq!(config.whisper_edge(), 4.0 * SpatialAudioConfig::PROXIMITY_FACTOR);
}

// Operators who already set the key keep their value; the Java mods' generated config mirror
// cannot carry an alias, so the key itself is what stays.
#[test]
fn an_operator_whisper_distance_is_read_from_its_existing_key() {
    let config: SpatialAudioConfig =
        serde_json::from_str(r#"{ "deafen_distance": 6.5 }"#).expect("deserialize");

    assert_eq!(config.whisper_distance, 6.5);
}
