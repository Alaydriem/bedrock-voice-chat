use bvc_client_lib::bedrock::proxy::session::handlers::SetHealthHandler;
use bvc_client_lib::bedrock::proxy::session::{BedrockPacketHandler, BedrockSessionState};
use common::bedrock_protocol::protocol::packets::generated::misc::set_health::SetHealthPacket;
use common::players::PlayerEnum;

fn apply(last: &mut Option<i32>, state: &mut BedrockSessionState, health: i32) {
    SetHealthHandler {
        last_known_health: last,
    }
    .handle(&SetHealthPacket { health }, state, None);
}

fn spectator(state: &BedrockSessionState) -> bool {
    let PlayerEnum::Minecraft(player) = state.to_player_enum() else {
        panic!("expected Minecraft variant");
    };
    player.spectator
}

// Death marks the session dead without needing an emitter, and a health packet above
// zero (respawn) clears it.
#[test]
fn death_marks_the_session_dead_and_respawn_clears_it() {
    let mut state = BedrockSessionState::new("Alice".into(), Some("xuid-1".into()));
    let mut last = Some(20);

    apply(&mut last, &mut state, 0);
    assert!(spectator(&state));

    apply(&mut last, &mut state, 20);
    assert!(!spectator(&state));
}
