use std::sync::Arc;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockSessionState;

pub trait BedrockPacketHandler {
    type Packet;
    fn handle(
        self,
        packet: &Self::Packet,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    );
}
