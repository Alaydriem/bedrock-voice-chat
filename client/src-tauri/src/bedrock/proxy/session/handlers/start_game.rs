use std::sync::Arc;

use common::bedrock_protocol::StartGamePacket;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

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
