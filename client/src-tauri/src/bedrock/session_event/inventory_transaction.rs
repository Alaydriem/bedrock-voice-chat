use std::sync::Arc;

use common::bedrock_protocol::protocol::types::transaction::TransactionData;
use common::bedrock_protocol::protocol::types::use_item_action_type::UseItemActionType;
use common::structs::game::Coordinate;
use common::structs::packet::BedrockEvent;
use log::debug;

use crate::bedrock::bvc_disc_nbt::BvcDiscNbt;
use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct InventoryTransactionHandler<'a> {
    pub beacon_cache: &'a JukeboxBeaconCache,
}

impl<'a> BedrockPacketHandler for InventoryTransactionHandler<'a> {
    type Packet = TransactionData;

    fn handle(
        self,
        data: &TransactionData,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };

        let use_item = match data {
            TransactionData::ItemUse(use_item) => use_item,
            _ => return,
        };

        if !matches!(use_item.action_type, UseItemActionType::ClickBlock) {
            return;
        }

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => {
                debug!("Skipping jukebox event: no world_uuid in session state");
                return;
            }
        };

        let block_key = (
            use_item.block_position.x,
            use_item.block_position.y,
            use_item.block_position.z,
        );
        let block_pos = Coordinate {
            x: use_item.block_position.x as f32,
            y: use_item.block_position.y as f32,
            z: use_item.block_position.z as f32,
        };
        let player_xuid = state.player_uuid().unwrap_or("").to_string();

        let audio_id = match BvcDiscNbt::extract_audio_id(&use_item.held_item.extra) {
            Some(id) => id,
            None => return,
        };

        debug!(
            "Bedrock proxy: emitting JukeboxInsert audio_id={} at ({},{},{})",
            audio_id, block_key.0, block_key.1, block_key.2
        );
        self.beacon_cache.note_insert_pending(&block_pos);
        emitter.try_send(
            BedrockEvent::JukeboxInsert {
                audio_id,
                block_pos,
                dimension: state.dimension(),
                player_xuid,
            },
            world_uuid,
        );
    }
}
