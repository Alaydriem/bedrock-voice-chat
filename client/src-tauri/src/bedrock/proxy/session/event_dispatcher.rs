use std::sync::Arc;

use common::bedrock_protocol::protocol::event::EventPacket;
use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::TextPacketBody;
use common::bedrock_protocol::{Direction, Event};

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::BvcpCodec;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::BedrockPlayerStateCache;
use crate::bedrock::proxy::session::{
    BedrockPacketHandler, ChangeDimensionHandler, DisconnectedHandler, DispatchOutcome,
    GameTypeHandler, PlaySoundHandler, PlayerAuthInputHandler, SetHealthHandler, StartGameHandler,
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

    fn bvcp_token(packet: &TextPacket) -> Option<String> {
        let message = match &packet.body {
            TextPacketBody::MessageOnly(body) => &body.message,
            TextPacketBody::AuthorAndMessage(body) => &body.message,
            TextPacketBody::MessageAndParams(body) => &body.message,
        };
        BvcpCodec::parse_bvcp(message)
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
            EventPacket::PlaySound(p) if matches!(direction, Direction::Clientbound) => {
                PlaySoundHandler {
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
            EventPacket::ChatMessage(p) if matches!(direction, Direction::Clientbound) => {
                if let Some(token) = Self::bvcp_token(p) {
                    if let Some(emitter) = emitter {
                        emitter.try_send_observed(token);
                    }
                }
                false
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::bedrock_protocol::ProtocolVersion;
    use common::bedrock_protocol::protocol::types::generated::{AuthorAndMessage, TextPacketType};
    use common::structs::packet::{PacketType, QuicNetworkPacketData};
    use crate::NetworkPacket;
    use crate::bedrock::proxy::presence::BvcpCodec;

    fn chat_event(message: &str, direction: Direction) -> Event {
        let packet = TextPacket {
            localize: false,
            body: TextPacketBody::AuthorAndMessage(AuthorAndMessage {
                message_type: TextPacketType::Chat,
                player_name: "bob".to_string(),
                message: message.to_string(),
            }),
            sender_s_xuid: "xuid".to_string(),
            platform_id: String::new(),
            filtered_message: None,
        };
        Event::new(
            ProtocolVersion::LATEST,
            direction,
            EventPacket::ChatMessage(packet),
        )
    }

    fn build_dispatcher(
    ) -> (BedrockSessionEventDispatcher, flume::Receiver<NetworkPacket>) {
        let (tx, rx) = flume::unbounded::<NetworkPacket>();
        let emitter = Arc::new(BedrockEventEmitter::new(Arc::new(tx)));
        let dispatcher = BedrockSessionEventDispatcher::new(
            "alice".to_string(),
            Arc::new(JukeboxBeaconCache::new()),
            Arc::new(BedrockPlayerStateCache::new()),
            Some(emitter),
        );
        (dispatcher, rx)
    }

    #[test]
    fn clientbound_bvcp_chat_emits_observed_token() {
        let (mut dispatcher, rx) = build_dispatcher();
        let mut state = BedrockSessionState::new("alice".to_string(), None);

        let evt = chat_event(&BvcpCodec::format_bvcp("tok-1"), Direction::Clientbound);
        dispatcher.dispatch(&evt, &mut state);

        let packet = rx.try_recv().expect("observed packet should be emitted");
        assert_eq!(packet.data.packet_type, PacketType::PeerPresenceObserved);
        match packet.data.data {
            QuicNetworkPacketData::PeerPresenceObserved(observed) => {
                assert_eq!(observed.token, "tok-1");
            }
            other => panic!("expected PeerPresenceObserved, got {:?}", other),
        }
    }

    #[test]
    fn clientbound_non_bvcp_chat_emits_nothing() {
        let (mut dispatcher, rx) = build_dispatcher();
        let mut state = BedrockSessionState::new("alice".to_string(), None);

        let evt = chat_event("hello world", Direction::Clientbound);
        dispatcher.dispatch(&evt, &mut state);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn serverbound_bvcp_chat_is_ignored() {
        let (mut dispatcher, rx) = build_dispatcher();
        let mut state = BedrockSessionState::new("alice".to_string(), None);

        let evt = chat_event(&BvcpCodec::format_bvcp("tok-1"), Direction::Serverbound);
        dispatcher.dispatch(&evt, &mut state);

        assert!(rx.try_recv().is_err());
    }
}
