use std::sync::Arc;

use common::bedrock_protocol::Event;
use common::structs::bedrock::AddonMode;

use crate::bedrock::BedrockChatChannel;
use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::BedrockPlayerStateCache;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::proxy::session::BedrockSessionState;
use crate::bedrock::proxy::session::DispatchOutcome;
use crate::bedrock::proxy::session::mode::{
    DispatchResult, FullDispatch, ModeDispatch, ModeDispatcher, RelayOnlyDispatch,
};

pub struct BedrockSessionEventDispatcher {
    player_name: String,
    player_state_cache: Arc<BedrockPlayerStateCache>,
    child: ModeDispatcher,
}

impl BedrockSessionEventDispatcher {
    pub fn new(
        player_name: String,
        beacon_cache: Arc<JukeboxBeaconCache>,
        player_state_cache: Arc<BedrockPlayerStateCache>,
        emitter: Option<Arc<BedrockEventEmitter>>,
        control_tx: crate::control::ControlActionSender,
        state_bus: crate::control::ControlStateBus,
        chat_channel: Option<Arc<BedrockChatChannel>>,
        mode: AddonMode,
    ) -> Self {
        // Built once rather than per event: `FullDispatch` carries per-session
        // trackers, and rebuilding it would reset first-input and death
        // detection on every packet.
        let child = match mode {
            AddonMode::Net => {
                ModeDispatcher::RelayOnly(RelayOnlyDispatch::new(player_name.clone()))
            }
            AddonMode::NoNet => ModeDispatcher::Full(FullDispatch::new(
                player_name.clone(),
                beacon_cache,
                emitter,
                control_tx,
                state_bus,
                chat_channel,
            )),
        };

        Self {
            player_name,
            player_state_cache,
            child,
        }
    }

    pub fn dispatch(&mut self, evt: &Event, state: &mut BedrockSessionState) -> DispatchOutcome {
        let DispatchResult {
            outcome,
            state_changed,
        } = self.child.dispatch(evt, state);

        if state_changed {
            self.player_state_cache
                .set(&self.player_name, state.to_player_enum());
        }

        outcome
    }
}
