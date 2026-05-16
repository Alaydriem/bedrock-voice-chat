use std::sync::Arc;

use common::bedrock_protocol::DisconnectReason;
use log::info;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::{BedrockPacketHandler, PlayerLeaveHandler};
use crate::bedrock::session_state::BedrockSessionState;

pub struct DisconnectedHandler<'a> {
    pub player_name: &'a str,
}

impl<'a> BedrockPacketHandler for DisconnectedHandler<'a> {
    type Packet = DisconnectReason;

    fn handle(
        self,
        reason: &DisconnectReason,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        info!(
            "Bedrock session disconnected for {}: {:?}",
            self.player_name, reason
        );
        PlayerLeaveHandler.handle(&(), state, emitter);
    }
}
