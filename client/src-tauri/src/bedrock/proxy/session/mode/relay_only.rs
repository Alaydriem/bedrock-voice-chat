use common::bedrock_protocol::Event;
use common::bedrock_protocol::protocol::event::EventPacket;

use super::{DispatchResult, ModeDispatch};
use crate::bedrock::proxy::session::BedrockSessionState;
use crate::bedrock::proxy::session::{
    BedrockPacketHandler, DisconnectedHandler, DispatchOutcome, StartGameHandler,
};

// Connection setup only. Everything else belongs to the world's addon on a net
// world, so carrying it here would duplicate a delivery that already happened.
pub struct RelayOnlyDispatch {
    player_name: String,
}

impl RelayOnlyDispatch {
    pub fn new(player_name: String) -> Self {
        Self { player_name }
    }
}

impl ModeDispatch for RelayOnlyDispatch {
    fn dispatch(
        &mut self,
        evt: &Event,
        state: &mut BedrockSessionState,
    ) -> DispatchResult<DispatchOutcome, bool> {
        match evt.packet() {
            EventPacket::StartGame(p) => {
                StartGameHandler.handle(p, state, None);
                DispatchResult {
                    outcome: DispatchOutcome::Continue,
                    state_changed: true,
                }
            }
            EventPacket::Disconnected(reason) => {
                DisconnectedHandler {
                    player_name: &self.player_name,
                }
                .handle(reason, state, None);
                DispatchResult {
                    outcome: DispatchOutcome::SessionEnded {
                        reason: "peer_disconnect",
                        detail: Some(format!("{:?}", reason)),
                    },
                    state_changed: false,
                }
            }
            _ => DispatchResult {
                outcome: DispatchOutcome::Continue,
                state_changed: false,
            },
        }
    }
}
