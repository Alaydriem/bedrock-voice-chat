use common::game_data::Dimension;
use common::request::audio::{AudioPlayRequest, GameAudioContext};

#[test]
fn a_jukebox_request_in_a_custom_dimension_deserializes() {
    let json = r#"{
        "audio_file_id": "disc-1",
        "game": {
            "game": "minecraft",
            "coordinates": {"x": 1, "y": 64, "z": -3},
            "dimension": "mypack:aether",
            "world_uuid": "w"
        }
    }"#;

    let request: AudioPlayRequest = serde_json::from_str(json).expect("jukebox request parses");

    let GameAudioContext::Minecraft(context) = request.game;
    assert_eq!(context.dimension, Dimension::parse("mypack:aether"));
}
