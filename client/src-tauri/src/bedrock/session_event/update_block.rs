use std::sync::Arc;

use common::bedrock_protocol::protocol::packets::generated::misc::update_block::UpdateBlockPacketAny;
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
    type Packet = UpdateBlockPacketAny;

    fn handle(
        self,
        packet: &UpdateBlockPacketAny,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };

        let (x, y, z) = match packet {
            UpdateBlockPacketAny::V897(p) => {
                (p.block_position.x, p.block_position.y, p.block_position.z)
            }
            UpdateBlockPacketAny::V944(p) => {
                (p.block_position.x, p.block_position.y, p.block_position.z)
            }
        };

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
