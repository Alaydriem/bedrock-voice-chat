use std::sync::Arc;

use common::bedrock_protocol::{Direction, Event};

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;
use crate::bedrock::player_state_cache::BedrockPlayerStateCache;
use crate::bedrock::session_event::{
    BedrockPacketHandler, ChangeDimensionHandler, DisconnectedHandler, DispatchOutcome,
    GameTypeHandler, InventoryTransactionHandler, PlayerAuthInputHandler, SetHealthHandler,
    StartGameHandler, UpdateBlockHandler,
};
use crate::bedrock::session_state::BedrockSessionState;

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
        let state_changed = match evt {
            Event::StartGame(_, p) => {
                StartGameHandler.handle(p, state, emitter);
                true
            }
            Event::PlayerAuthInput(dir, p) => {
                PlayerAuthInputHandler {
                    beacon_cache: &self.beacon_cache,
                    direction: dir,
                    player_auth_input_seen: &mut self.player_auth_input_seen,
                }
                .handle(p, state, emitter);
                true
            }
            Event::ChangeDimension(_, p) => {
                ChangeDimensionHandler.handle(p, state, emitter);
                true
            }
            Event::SetPlayerGameType(_, p) => {
                GameTypeHandler.handle(&p.gamemode, state, emitter);
                true
            }
            Event::UpdatePlayerGameType(_, p) => {
                GameTypeHandler.handle(&p.gamemode, state, emitter);
                true
            }
            Event::InventoryTransaction(Direction::Serverbound, p) => {
                InventoryTransactionHandler {
                    beacon_cache: &self.beacon_cache,
                }
                .handle(&p.transaction.data, state, emitter);
                false
            }
            Event::UpdateBlock(Direction::Clientbound, p) => {
                UpdateBlockHandler {
                    beacon_cache: &self.beacon_cache,
                }
                .handle(p, state, emitter);
                false
            }
            Event::SetHealth(_, p) => {
                SetHealthHandler {
                    last_known_health: &mut self.last_known_health,
                }
                .handle(p, state, emitter);
                false
            }
            Event::Disconnected(reason) => {
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
