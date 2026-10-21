use std::sync::Arc;

use common::structs::packet::BedrockEvent;
use tauri_plugin_curia::curia;

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

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

        curia::info!("Bedrock proxy: emitting player leave", {
            player: state.name().to_string(),
        });
        emitter.try_send_position(state.to_departed_player_enum());

        let event = BedrockEvent::PlayerLeave {
            player_xuid: state.player_uuid().unwrap_or("").to_string(),
        };
        emitter.try_send(event, world_uuid);
    }
}
