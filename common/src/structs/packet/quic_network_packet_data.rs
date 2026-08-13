use serde::{Deserialize, Serialize};

use super::audio_frame_packet::AudioFramePacket;
use super::bedrock_event_packet::BedrockEventPacket;
use super::channel_event_packet::ChannelEventPacket;
use super::chat_message_packet::ChatMessagePacket;
use super::chat_rejected_packet::ChatRejectedPacket;
use super::chat_send_packet::ChatSendPacket;
use super::client_action_packet::ClientActionPacket;
use super::player_preference_packet::PlayerPreferencePacket;
use super::query_state_packet::QueryStatePacket;
use super::collection_packet::CollectionPacket;
use super::debug_packet::DebugPacket;
use super::health_check_packet::HealthCheckPacket;
use super::peer_announce_inject_packet::PeerAnnounceInjectPacket;
use super::peer_announce_observed_packet::PeerAnnounceObservedPacket;
use super::peer_presence_inject_packet::PeerPresenceInjectPacket;
use super::peer_presence_observed_packet::PeerPresenceObservedPacket;
use super::player_data_packet::PlayerDataPacket;
use super::player_position_packet::PlayerPositionPacket;
use super::player_presence_event::PlayerPresenceEvent;
use super::server_error_packet::ServerErrorPacket;
use crate::structs::relay::{AudioAvailable, AudioQuery};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuicNetworkPacketData {
    AudioFrame(AudioFramePacket),
    PlayerData(PlayerDataPacket),
    PlayerPosition(PlayerPositionPacket),
    ChannelEvent(ChannelEventPacket),
    ChatMessage(ChatMessagePacket),
    ChatSend(ChatSendPacket),
    Collection(CollectionPacket),
    Debug(DebugPacket),
    PlayerPresence(PlayerPresenceEvent),
    ServerError(ServerErrorPacket),
    HealthCheck(HealthCheckPacket),
    BedrockEvent(BedrockEventPacket),
    PeerPresenceInject(PeerPresenceInjectPacket),
    PeerPresenceObserved(PeerPresenceObservedPacket),
    PeerAnnounceInject(PeerAnnounceInjectPacket),
    PeerAnnounceObserved(PeerAnnounceObservedPacket),
    AudioQuery(AudioQuery),
    AudioAvailable(AudioAvailable),
    ClientAction(ClientActionPacket),
    QueryState(QueryStatePacket),
    PlayerPreference(PlayerPreferencePacket),
    // Appended, never inserted: postcard encodes the variant index, so a new variant in the
    // middle shifts every later discriminant and mis-decodes packets that were fine before.
    ChatRejected(ChatRejectedPacket),
}
