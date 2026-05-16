use std::sync::Arc;

use common::bedrock_protocol::UpdateBlockPacket;
use common::structs::packet::BedrockEvent;
use log::debug;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct UpdateBlockHandler<'a> {
    pub beacon_cache: &'a JukeboxBeaconCache,
}

impl<'a> BedrockPacketHandler for UpdateBlockHandler<'a> {
    type Packet = UpdateBlockPacket;

    fn handle(
        self,
        packet: &UpdateBlockPacket,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };

        let x = packet.position.x;
        let y = packet.position.y;
        let z = packet.position.z;

        let event_id = match self.beacon_cache.process_update_block((x, y, z)) {
            Some(id) => id,
            None => return,
        };

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => {
                debug!("Skipping JukeboxEject: UpdateBlock at cached jukebox but no world_uuid");
                return;
            }
        };

        debug!(
            "Bedrock proxy: emitting JukeboxEject event_id={} at ({},{},{})",
            event_id, x, y, z
        );
        emitter.try_send(
            BedrockEvent::JukeboxEject {
                event_id,
                player_xuid: state.player_uuid().unwrap_or("").to_string(),
            },
            world_uuid,
        );
    }
}
