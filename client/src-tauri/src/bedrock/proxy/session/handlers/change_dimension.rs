use std::sync::Arc;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

pub struct ChangeDimensionHandler;

impl BedrockPacketHandler for ChangeDimensionHandler {
    type Packet = i32;

    fn handle(
        self,
        dimension: &i32,
        state: &mut BedrockSessionState,
        _emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        state.apply_change_dimension(*dimension);
    }
}
