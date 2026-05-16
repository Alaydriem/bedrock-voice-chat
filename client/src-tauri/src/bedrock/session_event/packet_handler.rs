use std::sync::Arc;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_state::BedrockSessionState;

pub trait BedrockPacketHandler {
    type Packet;
    fn handle(
        self,
        packet: &Self::Packet,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    );
}
