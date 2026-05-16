use std::sync::Arc;

use common::bedrock_protocol::{Direction, PlayerAuthInputPacket};
use log::debug;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;
use crate::bedrock::session_event::{BedrockPacketHandler, InventoryTransactionHandler};
use crate::bedrock::session_state::BedrockSessionState;

pub struct PlayerAuthInputHandler<'a> {
    pub beacon_cache: &'a JukeboxBeaconCache,
    pub direction: &'a Direction,
    pub player_auth_input_seen: &'a mut bool,
}

impl<'a> BedrockPacketHandler for PlayerAuthInputHandler<'a> {
    type Packet = PlayerAuthInputPacket;

    fn handle(
        self,
        packet: &PlayerAuthInputPacket,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        if !*self.player_auth_input_seen {
            debug!(
                "Bedrock: first PlayerAuthInput received dir={:?}",
                self.direction
            );
            *self.player_auth_input_seen = true;
        }
        state.apply_position(packet);
        if matches!(self.direction, Direction::Serverbound) {
            if let Some(tx) = &packet.transaction {
                InventoryTransactionHandler {
                    beacon_cache: self.beacon_cache,
                }
                .handle(&tx.data, state, emitter);
            }
        }
    }
}
