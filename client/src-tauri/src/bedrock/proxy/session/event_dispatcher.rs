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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NetworkPacket;
    use crate::bedrock::proxy::presence::BvcpCodec;
    use common::bedrock_protocol::Direction;
    use common::bedrock_protocol::ProtocolVersion;
    use common::bedrock_protocol::protocol::event::EventPacket;
    use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
    use common::bedrock_protocol::protocol::types::generated::{
        AuthorAndMessage, TextPacketBody, TextPacketType,
    };
    use common::structs::packet::{PacketType, QuicNetworkPacketData};

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

    // These exercise the observation and chat-ingress arms, which only the
    // full-processing child owns.
    fn build_dispatcher() -> (
        BedrockSessionEventDispatcher,
        flume::Receiver<NetworkPacket>,
    ) {
        let (tx, rx) = flume::unbounded::<NetworkPacket>();
        let emitter = Arc::new(BedrockEventEmitter::new(Arc::new(tx)));
        let dispatcher = BedrockSessionEventDispatcher::new(
            "alice".to_string(),
            Arc::new(JukeboxBeaconCache::new()),
            Arc::new(BedrockPlayerStateCache::new()),
            Some(emitter),
            crate::control::ControlActionSender::channel().0,
            crate::control::ControlStateBus::new(),
            None,
            AddonMode::NoNet,
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
    fn clientbound_bvca_chat_emits_announce_observed() {
        let (mut dispatcher, rx) = build_dispatcher();
        let mut state = BedrockSessionState::new("alice".to_string(), None);
        state.set_world_uuid_for_test("world-xyz".to_string());

        let evt = chat_event(
            &BvcpCodec::format_bvca("peer.example:443"),
            Direction::Clientbound,
        );
        dispatcher.dispatch(&evt, &mut state);

        let packet = rx
            .try_recv()
            .expect("announce observed packet should be emitted");
        assert_eq!(packet.data.packet_type, PacketType::PeerAnnounceObserved);
        match packet.data.data {
            QuicNetworkPacketData::PeerAnnounceObserved(obs) => {
                assert_eq!(obs.hashed_world, "world-xyz");
                assert_eq!(obs.endpoint, "peer.example:443");
            }
            other => panic!("expected PeerAnnounceObserved, got {:?}", other),
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
