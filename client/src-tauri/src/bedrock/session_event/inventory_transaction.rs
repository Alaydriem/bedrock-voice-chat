use std::sync::Arc;

use common::bedrock_protocol::protocol::types::generated::{
    InventoryTransactionPacketTransaction, ItemUseActionType,
};
use common::structs::game::BlockCoordinate;
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
    type Packet = InventoryTransactionPacketTransaction;

    fn handle(
        self,
        data: &InventoryTransactionPacketTransaction,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let use_item = match data {
            InventoryTransactionPacketTransaction::ItemUseInventoryTransaction(tx) => tx,
            _ => return,
        };

        if !matches!(use_item.action_type, ItemUseActionType::Place) {
            return;
        }

        Self::emit_jukebox_insert(
            self.beacon_cache,
            state,
            emitter,
            BlockCoordinate::new(
                use_item.position.x,
                use_item.position.y,
                use_item.position.z,
            ),
            use_item.item.user_data_buffer.as_bytes(),
        );
    }
}

impl<'a> InventoryTransactionHandler<'a> {
    pub(crate) fn emit_jukebox_insert(
        beacon_cache: &JukeboxBeaconCache,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
        block_pos: BlockCoordinate,
        nbt_bytes: &[u8],
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => {
                debug!("Skipping jukebox event: no world_uuid in session state");
                return;
            }
        };

        let player_xuid = state.player_uuid().unwrap_or("").to_string();

        let audio_id = match BvcDiscNbt::extract_audio_id_bytes(nbt_bytes) {
            Some(id) => id,
            None => return,
        };

        debug!(
            "Bedrock proxy: emitting JukeboxInsert audio_id={} at ({},{},{})",
            audio_id, block_pos.x, block_pos.y, block_pos.z
        );
        beacon_cache.note_insert_pending(block_pos);
        emitter.try_send(
            BedrockEvent::JukeboxInsert {
                audio_id,
                block_pos: block_pos.into(),
                dimension: state.dimension(),
                player_xuid,
            },
            world_uuid,
        );
    }
}
