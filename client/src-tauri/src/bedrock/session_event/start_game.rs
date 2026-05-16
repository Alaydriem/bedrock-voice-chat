use std::sync::Arc;

use common::bedrock_protocol::StartGamePacket;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct StartGameHandler;

impl BedrockPacketHandler for StartGameHandler {
    type Packet = StartGamePacket;

    fn handle(
        self,
        packet: &StartGamePacket,
        state: &mut BedrockSessionState,
        _emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        state.apply_start_game(packet);
    }
}
