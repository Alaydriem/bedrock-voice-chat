use std::sync::Arc;

use common::bedrock_protocol::protocol::event::EventPacket;
use common::bedrock_protocol::{Direction, Event};
use log::info;

use super::{DispatchResult, ModeDispatch};
use crate::bedrock::BedrockChatChannel;
use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::ChatCodec;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::proxy::session::BedrockSessionState;
use crate::bedrock::proxy::session::{
    BedrockPacketHandler, ChangeDimensionHandler, DisconnectedHandler, DispatchOutcome,
    GameTypeHandler, PlaySoundHandler, PlayerAuthInputHandler, SetHealthHandler, StartGameHandler,
};

// The proxy carries everything: positions, dimension and gamemode changes, the
// bvc: sound buses, deaths, presence and announce observations, and chat in both
// directions. Correct only where the world's addon has no HTTP channel of its
// own.
pub struct FullDispatch {
    player_name: String,
    beacon_cache: Arc<JukeboxBeaconCache>,
    emitter: Option<Arc<BedrockEventEmitter>>,
    control_tx: crate::control::ControlActionSender,
    state_bus: crate::control::ControlStateBus,
    chat_channel: Option<Arc<BedrockChatChannel>>,
    last_known_health: Option<i32>,
    player_auth_input_seen: bool,
}

impl FullDispatch {
    pub fn new(
        player_name: String,
        beacon_cache: Arc<JukeboxBeaconCache>,
        emitter: Option<Arc<BedrockEventEmitter>>,
        control_tx: crate::control::ControlActionSender,
        state_bus: crate::control::ControlStateBus,
        chat_channel: Option<Arc<BedrockChatChannel>>,
    ) -> Self {
        Self {
            player_name,
            beacon_cache,
            emitter,
            control_tx,
            state_bus,
            chat_channel,
            last_known_health: None,
            player_auth_input_seen: false,
        }
    }

}

impl ModeDispatch for FullDispatch {
    fn dispatch(
        &mut self,
        evt: &Event,
        state: &mut BedrockSessionState,
    ) -> DispatchResult<DispatchOutcome, bool> {
        let emitter = self.emitter.as_ref();
        let direction = evt.direction();
        let state_changed = match evt.packet() {
            EventPacket::StartGame(p) => {
                StartGameHandler.handle(p, state, emitter);
                true
            }
            EventPacket::PlayerAuthInput(p) => {
                PlayerAuthInputHandler {
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
                info!("Received UpdatePlayerGameType: {:?}", p);
                let gamemode = i64::from(p.player_game_type) as i32;
                GameTypeHandler.handle(&gamemode, state, emitter);
                true
            }
            // The bvc: buses are server-authored (/playsound is clientbound);
            // like the ChatMessage arm, ignore the serverbound direction.
            EventPacket::PlaySound(p) if matches!(direction, Direction::Clientbound) => {
                info!("Received PlaySound: {:?}", p);
                PlaySoundHandler {
                    beacon_cache: &self.beacon_cache,
                    player_name: &self.player_name,
                    control_tx: self.control_tx.clone(),
                    state_bus: self.state_bus.clone(),
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
                return DispatchResult {
                    outcome: DispatchOutcome::SessionEnded {
                        reason: "peer_disconnect",
                        detail: Some(format!("{:?}", reason)),
                    },
                    state_changed: false,
                };
            }
            EventPacket::ChatMessage(p) if matches!(direction, Direction::Clientbound) => {
                if let Some(chat) = self.chat_channel.as_ref() {
                    // Everything the realm broadcasts that a person should read. ChatCodec
                    // rejects rides itself — relying on caller ordering for a security
                    // boundary is how leaks happen.
                    if let Some(line) = ChatCodec::decode(p) {
                        chat.emit(line);
                    }
                }
                false
            }
            _ => false,
        };

        DispatchResult {
            outcome: DispatchOutcome::Continue,
            state_changed,
        }
    }
}
