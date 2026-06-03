use std::sync::Arc;

use common::bedrock_protocol::protocol::packets::generated::misc::change_dimension::ChangeDimensionPacket;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

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
