use std::sync::Arc;

use common::bedrock_protocol::DisconnectReason;
use log::info;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockSessionState;
use crate::bedrock::proxy::session::{BedrockPacketHandler, PlayerLeaveHandler};

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
