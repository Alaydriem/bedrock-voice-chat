use serde::{Deserialize, Serialize};

use super::audio_frame_packet::AudioFramePacket;
use super::player_data_packet::PlayerDataPacket;
use super::channel_event_packet::ChannelEventPacket;
use super::collection_packet::CollectionPacket;
use super::debug_packet::DebugPacket;
use super::player_presence_event::PlayerPresenceEvent;
use super::server_error_packet::ServerErrorPacket;
use super::health_check_packet::HealthCheckPacket;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuicNetworkPacketData {
    AudioFrame(AudioFramePacket),
    PlayerData(PlayerDataPacket),
    ChannelEvent(ChannelEventPacket),
    Collection(CollectionPacket),
    Debug(DebugPacket),
    PlayerPresence(PlayerPresenceEvent),
    ServerError(ServerErrorPacket),
    HealthCheck(HealthCheckPacket),
}
