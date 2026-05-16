use std::sync::Arc;

use common::structs::packet::BedrockEvent;
use log::info;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct PlayerLeaveHandler;

impl BedrockPacketHandler for PlayerLeaveHandler {
    type Packet = ();

    fn handle(
        self,
        _: &(),
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };
        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => return,
        };

        let event = BedrockEvent::PlayerLeave {
            player_xuid: state.player_uuid().unwrap_or("").to_string(),
        };
        info!("Bedrock proxy: emitting player leave for {}", state.name());
        emitter.try_send(event, world_uuid);
    }
}
