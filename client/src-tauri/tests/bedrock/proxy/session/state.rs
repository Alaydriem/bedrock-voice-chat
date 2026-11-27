use bvc_client_lib::bedrock::proxy::session::BedrockSessionState;
use common::game_data::{Dimension, DimensionHash};
use common::players::PlayerEnum;

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

// The position heartbeat re-sends the session every tick; a dead player must stay a
// non-whispering spectator until respawn.
#[test]
fn a_dead_session_reports_spectator_and_not_whispering_until_alive_again() {
    let mut state = session();

    state.set_dead(true);
    let PlayerEnum::Minecraft(player) = state.to_player_enum() else {
        panic!("expected Minecraft variant");
    };
    assert!(player.spectator);
    assert!(!player.whispering);

    state.set_dead(false);
    let PlayerEnum::Minecraft(player) = state.to_player_enum() else {
        panic!("expected Minecraft variant");
    };
    assert!(!player.spectator);
}

// A departed player is parked in a dimension no live player occupies, so the server
// stops routing voice to or from them.
#[test]
fn a_departed_player_is_unrouted() {
    let PlayerEnum::Minecraft(player) = session().to_departed_player_enum() else {
        panic!("expected Minecraft variant");
    };
    assert_eq!(player.dimension, Dimension::Unrouted);
}
