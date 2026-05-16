use std::sync::Arc;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct GameTypeHandler;

impl BedrockPacketHandler for GameTypeHandler {
    type Packet = i32;

    fn handle(
        self,
        gamemode: &i32,
        state: &mut BedrockSessionState,
        _emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        state.apply_game_type(*gamemode);
    }
}
