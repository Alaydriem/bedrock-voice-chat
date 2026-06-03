use std::sync::Arc;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

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
