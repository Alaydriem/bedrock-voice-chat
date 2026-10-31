use bvc_client_lib::bedrock::proxy::session::BedrockSessionState;
use common::game_data::{Dimension, DimensionHash};

fn session() -> BedrockSessionState {
    BedrockSessionState::new("Alice".to_string(), Some("xuid-1".to_string()))
}

// An add-on dimension has an id past the three vanilla ones. Merged into Overworld, a
// player standing in it would hear and be heard by Overworld players at the same
// coordinates.
#[test]
fn an_unknown_dimension_id_is_isolated_not_overworld() {
    let mut state = session();

    state.apply_change_dimension(7);
    assert_eq!(
        state.dimension(),
        Dimension::Other(DimensionHash::of("bedrock:7"))
    );

    state.apply_change_dimension(1);
    assert_eq!(state.dimension(), Dimension::TheNether);
}
