use std::sync::Arc;

use common::bedrock_protocol::ChangeDimensionPacket;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct ChangeDimensionHandler;

impl BedrockPacketHandler for ChangeDimensionHandler {
    type Packet = ChangeDimensionPacket;

    fn handle(
        self,
        packet: &ChangeDimensionPacket,
        state: &mut BedrockSessionState,
        _emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        state.apply_change_dimension(packet);
    }
}
