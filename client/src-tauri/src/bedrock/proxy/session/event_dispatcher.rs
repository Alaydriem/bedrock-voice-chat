use std::sync::Arc;

use common::bedrock_protocol::protocol::event::EventPacket;
use common::bedrock_protocol::{Direction, Event};

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::BedrockPlayerStateCache;
use crate::bedrock::proxy::session::{
    BedrockPacketHandler, ChangeDimensionHandler, DisconnectedHandler, DispatchOutcome,
    GameTypeHandler, InventoryTransactionHandler, PlayerAuthInputHandler, SetHealthHandler,
    StartGameHandler, UpdateBlockHandler,
};
use crate::bedrock::proxy::session::BedrockSessionState;

pub struct BedrockSessionEventDispatcher {
    player_name: String,
    beacon_cache: Arc<JukeboxBeaconCache>,
    player_state_cache: Arc<BedrockPlayerStateCache>,
    emitter: Option<Arc<BedrockEventEmitter>>,
    last_known_health: Option<i32>,
    player_auth_input_seen: bool,
}

impl BedrockSessionEventDispatcher {
    pub fn new(
        player_name: String,
        beacon_cache: Arc<JukeboxBeaconCache>,
        player_state_cache: Arc<BedrockPlayerStateCache>,
        emitter: Option<Arc<BedrockEventEmitter>>,
    ) -> Self {
        Self {
            player_name,
            beacon_cache,
            player_state_cache,
            emitter,
            last_known_health: None,
            player_auth_input_seen: false,
        }
    }

    pub fn dispatch(
        &mut self,
        evt: &Event,
        state: &mut BedrockSessionState,
    ) -> DispatchOutcome {
        let emitter = self.emitter.as_ref();
        let direction = evt.direction();
        let state_changed = match evt.packet() {
            EventPacket::StartGame(p) => {
                StartGameHandler.handle(p, state, emitter);
                true
            }
            EventPacket::PlayerAuthInput(p) => {
                PlayerAuthInputHandler {
                    beacon_cache: &self.beacon_cache,
                    direction: &direction,
                    player_auth_input_seen: &mut self.player_auth_input_seen,
                }
                .handle(p, state, emitter);
                true
            }
            EventPacket::ChangeDimension(p) => {
                ChangeDimensionHandler.handle(p, state, emitter);
                true
            }
            EventPacket::SetPlayerGameType(p) => {
                let gamemode = i64::from(p.player_game_type) as i32;
                GameTypeHandler.handle(&gamemode, state, emitter);
                true
            }
            EventPacket::UpdatePlayerGameType(p) => {
                let gamemode = i64::from(p.player_game_type) as i32;
                GameTypeHandler.handle(&gamemode, state, emitter);
                true
            }
            EventPacket::InventoryTransaction(p) if matches!(direction, Direction::Serverbound) => {
                InventoryTransactionHandler {
                    beacon_cache: &self.beacon_cache,
                }
                .handle(&p.transaction, state, emitter);
                false
            }
            EventPacket::UpdateBlock(p) if matches!(direction, Direction::Clientbound) => {
                UpdateBlockHandler {
                    beacon_cache: &self.beacon_cache,
                }
                .handle(p, state, emitter);
                false
            }
            EventPacket::SetHealth(p) => {
                SetHealthHandler {
                    last_known_health: &mut self.last_known_health,
                }
                .handle(p, state, emitter);
                false
            }
            EventPacket::Disconnected(reason) => {
                DisconnectedHandler {
                    player_name: &self.player_name,
                }
                .handle(reason, state, emitter);
                return DispatchOutcome::SessionEnded {
                    reason: "peer_disconnect",
                    detail: Some(format!("{:?}", reason)),
                };
            }
            _ => false,
        };
        if state_changed {
            self.player_state_cache
                .set(&self.player_name, state.to_player_enum());
        }
        DispatchOutcome::Continue
    }
}
