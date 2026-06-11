use std::sync::Arc;

use common::bedrock_protocol::PlayerAuthInputPacket;
use log::debug;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

pub struct PlayerAuthInputHandler<'a> {
    pub player_auth_input_seen: &'a mut bool,
}

impl<'a> BedrockPacketHandler for PlayerAuthInputHandler<'a> {
    type Packet = PlayerAuthInputPacket;

    fn handle(
        self,
        packet: &PlayerAuthInputPacket,
        state: &mut BedrockSessionState,
        _emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        if !*self.player_auth_input_seen {
            debug!("Bedrock: first PlayerAuthInput received");
            *self.player_auth_input_seen = true;
        }
        state.apply_position(packet);
    }
}
