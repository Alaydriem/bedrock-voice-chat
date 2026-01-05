use anyhow::{anyhow, Error};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

use super::channel::ChannelEvents;
use moka::future::Cache;
use std::sync::Arc;

/// The packet type
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    ChannelEvent,
    Collection,
    Debug,
    PlayerPresence,
    ServerError,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PacketOwner {
    pub name: String,
    pub client_id: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuicNetworkPacketData {
    AudioFrame(AudioFramePacket),
    PlayerData(PlayerDataPacket),
    ChannelEvent(ChannelEventPacket),
    Collection(CollectionPacket),
    Debug(DebugPacket),
    PlayerPresence(PlayerPresenceEvent),
    ServerError(ServerErrorPacket),
}

/// A Quic Network Datagram
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub owner: Option<PacketOwner>,
    pub data: QuicNetworkPacketData,
}

pub const MAX_DATAGRAM_SIZE: usize = 1150;

impl QuicNetworkPacket {
    /// Serialize the packet for direct QUIC DATAGRAM transmission (no framing)
    pub fn to_datagram(&self) -> Result<Vec<u8>, anyhow::Error> {
        let bytes = postcard::to_stdvec(&self)?;
        if bytes.len() > MAX_DATAGRAM_SIZE {
            return Err(anyhow!(
                "Serialized datagram size {} exceeds max {}",
                bytes.len(),
                MAX_DATAGRAM_SIZE
            ));
        }
        Ok(bytes)
    }

    /// Deserialize a packet from a QUIC DATAGRAM payload
    pub fn from_datagram(data: &[u8]) -> Result<Self, anyhow::Error> {
        if data.len() > MAX_DATAGRAM_SIZE {
            return Err(anyhow!(
                "Incoming datagram size {} exceeds max {}",
                data.len(),
                MAX_DATAGRAM_SIZE
            ));
        }
        postcard::from_bytes::<QuicNetworkPacket>(data)
            .map_err(|e| anyhow!("Postcard deserialization error: {}", e))
    }

    /// Returns the packet type
    pub fn get_packet_type(&self) -> PacketType {
        self.packet_type.clone()
    }

    /// Whether or not a packet should be broadcasted
    pub fn is_broadcast(&self) -> bool {
        match self.packet_type {
            PacketType::AudioFrame => false,
            PacketType::Debug => true,
            PacketType::PlayerData => true,
            PacketType::ChannelEvent => true,
            PacketType::Collection => false,
            PacketType::PlayerPresence => true,
            PacketType::ServerError => false,
        }
    }

    /// Returns the author
    pub fn get_author(&self) -> String {
        match &self.owner {
            Some(owner) => {
                // If the owner name is empty, or comes from the API, then default to the client ID
                if owner.name.eq(&"") || owner.name.eq(&"api") {
                    return general_purpose::STANDARD.encode(&owner.client_id);
                }

                return owner.name.clone();
            }
            None => String::from(""),
        }
    }

    /// Returns the client id
    pub fn get_client_id(&self) -> Vec<u8> {
        match &self.owner {
            Some(owner) => owner.client_id.clone(),
            None => vec![],
        }
    }

    /// Returns the underlying data frame.
    pub fn get_data(&self) -> Option<&QuicNetworkPacketData> {
        Some(&self.data)
    }

    // Updates the coordinates for a given packet with the player position data
    pub async fn update_coordinates(&mut self, player_data: Arc<Cache<String, crate::PlayerEnum>>) {
        match self.get_packet_type() {
            PacketType::AudioFrame => match self.get_data() {
                Some(data) => {
                    let data = data.to_owned();
                    let data: Result<AudioFramePacket, ()> = data.try_into();

                    match data {
                        Ok(mut data) => {
                            if data.sender.is_none() {
                                if let Some(sender_player) = player_data.get(&self.get_author()).await {
                                    data.sender = Some(sender_player);
                                    let audio_frame: QuicNetworkPacketData =
                                        QuicNetworkPacketData::AudioFrame(data);
                                    self.data = audio_frame;
                                }
                            }
                        },
                        Err(_) => {
                            tracing::error!("Could not downcast reference packet to audio frame");
                        }
                    }
                }
                None => {
                    tracing::error!("Could not downcast reference packet to audio frame");
                }
            },
            _ => {}
        }
    }

    /// To RON String
    pub fn to_string(&self) -> Result<String, Error> {
        match ron::to_string(&self) {
            Ok(message) => Ok(message),
            Err(e) => {
                tracing::error!(
                    "Could not convert QuicNetworkPacket back to String {}",
                    e.to_string()
                );
                Err(anyhow!(e.to_string()))
            }
        }
    }

    /// Converts a string to a QuicNetworkPacket, if possible
    pub fn from_string(message: String) -> Option<QuicNetworkPacket> {
        return match ron::from_str::<QuicNetworkPacket>(&String::from_utf8_lossy(
            message.as_bytes(),
        )) {
            Ok(packet) => Some(packet),
            Err(e) => {
                tracing::error!(
                    "Could not decode QuicNetworkPacket from string {}",
                    e.to_string()
                );
                None
            }
        };
    }

    /// Helper function to get all channels a player is in
    async fn get_player_channels(
        player_name: &str,
        channel_membership: &Cache<String, std::collections::HashSet<String>>,
    ) -> Vec<String> {
        let mut player_channels = Vec::new();
        for (channel_id, members) in channel_membership.iter() {
            if members.contains(player_name) {
                player_channels.push((*channel_id).clone());
            }
        }
        player_channels
    }

    /// Determines if a given PacketOwner can receive this QuicNetworkPacket
    pub async fn is_receivable(
        &mut self,
        recipient: PacketOwner,
        channel_membership: Arc<Cache<String, std::collections::HashSet<String>>>,
        position_data: Arc<Cache<String, crate::PlayerEnum>>,
        range: f32,
    ) -> bool {
        match self.get_packet_type() {
            PacketType::AudioFrame => match self.get_data() {
                Some(data) => match data.to_owned().try_into() {
                    Ok(data) => {
                        let current_player = &self.get_author();
                        let mut data: AudioFramePacket = data;

                        // You cannot receive your own audio packets
                        if current_player.eq(&recipient.name) {
                            return false;
                        }

                        // Check if both players are in the same channel using optimized approach
                        let receiver_name = &recipient.name;

                        // Get channels for both players concurrently
                        let (sender_channels, receiver_channels) = tokio::join!(
                            Self::get_player_channels(current_player, &channel_membership),
                            Self::get_player_channels(receiver_name, &channel_membership)
                        );

                        // Check if they share any channels (O(k*m) where k,m = channels per player, typically 1-2)
                        let players_in_same_channel = sender_channels.iter()
                            .any(|channel| receiver_channels.contains(channel));

                        // If the players are both in the same group, then the audio packet may be received
                        if players_in_same_channel {
                            // Group audio packets defer to client sending settings, and non-spatial by default
                            if data.spatial.is_none() {
                                // Group audio packets are non-spatial
                                data.spatial = Some(false);
                            }
                            self.data = QuicNetworkPacketData::AudioFrame(data);
                            return true;
                        }

                        // Get sender and recipient PlayerEnum
                        let (actual_sender, actual_recipient) = tokio::join!(
                            async {
                                // Try sender field from packet first, then cache
                                if let Some(sender) = data.sender.clone() {
                                    Some(sender)
                                } else {
                                    position_data.get(current_player).await
                                }
                            },
                            position_data.get(receiver_name)
                        );

                        // Use can_communicate_with() for spatial logic
                        if let (Some(sender), Some(recipient)) = (&actual_sender, &actual_recipient) {
                            if let Err(e) = sender.can_communicate_with(recipient, range) {
                                tracing::debug!("Audio packet rejected: {}", e);
                                return false;
                            }

                            // Handle spatial flag (Note: Some(false) rejected here because we're NOT in a channel)
                            match data.spatial {
                                Some(true) => {
                                    self.data = QuicNetworkPacketData::AudioFrame(data);
                                    return true;
                                }
                                Some(false) => return false,
                                None => {
                                    data.spatial = Some(true);
                                    self.data = QuicNetworkPacketData::AudioFrame(data);
                                    return true;
                                }
                            }
                        }

                        tracing::debug!("Could not find position data for sender or recipient");
                        // Fallback: sender or recipient not in cache
                        // Only allow explicitly non-spatial audio (matches current behavior)
                        match data.spatial {
                            Some(false) => return true,
                            _ => return false,
                        }
                    }
                    Err(_) => {
                        tracing::error!("Failed to decode audio frame data");
                        return false;
                    }
                },
                None => false,
            },
            // Player data should always be received
            PacketType::PlayerData => true,
            // Player presence events should always be received by all clients
            PacketType::PlayerPresence => true,
            PacketType::ServerError => match self.get_data() {
                Some(data) => match data.to_owned().try_into() {
                    Ok(data) => {
                        let mut _data: ServerErrorPacket = data;
                        self.get_author().eq(&recipient.name)
                    }
                    Err(_) => false,
                },
                None => false,
            },
            PacketType::ChannelEvent => true,
            // If there are other packet types we want recipiants to receive, this should be updated
            _ => self.is_broadcast(),
        }
    }
}

// A collection of audio frames that occur simultaniously
// The client is responsible for mixing and handling audio data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollectionPacket {
    pub data: Vec<QuicNetworkPacket>,
}

impl TryFrom<QuicNetworkPacketData> for CollectionPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::Collection(c) => Ok(c),
            _ => Err(()),
        }
    }
}

/// A single, Opus encoded audio frame
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    // Store pre-encoded zigzag+varint bytes for efficient serialization
    #[serde(with = "serde_bytes")]
    encoded_length: Vec<u8>, // zigzag+varint encoded i32

    #[serde(with = "serde_bytes")]
    encoded_timestamp: Vec<u8>, // zigzag+varint encoded i64

    pub sample_rate: u32,

    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,

    pub sender: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
}

impl TryFrom<QuicNetworkPacketData> for AudioFramePacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::AudioFrame(c) => Ok(c),
            _ => Err(()),
        }
    }
}

impl AudioFramePacket {
    /// Create a new AudioFramePacket with automatic timestamp and length encoding
    pub fn new(
        data: Vec<u8>,
        sample_rate: u32,
        sender: Option<crate::PlayerEnum>,
        spatial: Option<bool>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let length = data.len() as i32;

        Self {
            encoded_length: crate::encoding::encode_zigzag_varint_i32(length),
            encoded_timestamp: crate::encoding::encode_zigzag_varint_i64(timestamp),
            sample_rate,
            data,
            sender,
            spatial,
        }
    }

    /// Get the decoded length value
    pub fn length(&self) -> i32 {
        crate::encoding::decode_zigzag_varint_i32(&self.encoded_length)
            .unwrap_or((0, 0))
            .0
    }

    /// Get the decoded timestamp value (Unix timestamp in milliseconds)
    pub fn timestamp(&self) -> i64 {
        crate::encoding::decode_zigzag_varint_i64(&self.encoded_timestamp)
            .unwrap_or((0, 0))
            .0
    }

    /// Get the actual data length (convenience method)
    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    /// Get the size of the encoded length field (for space analysis)
    pub fn encoded_length_size(&self) -> usize {
        self.encoded_length.len()
    }

    /// Get the size of the encoded timestamp field (for space analysis)
    pub fn encoded_timestamp_size(&self) -> usize {
        self.encoded_timestamp.len()
    }
}

/// General Player Positioning data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerDataPacket {
    pub players: Vec<crate::PlayerEnum>,
}

impl TryFrom<QuicNetworkPacketData> for PlayerDataPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerData(c) => Ok(c),
            _ => Err(()),
        }
    }
}

/// Debug Packet
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket {
    pub owner: String,
    pub version: String,
    pub timestamp: u64,
}

impl TryFrom<QuicNetworkPacketData> for DebugPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::Debug(c) => Ok(c),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelEventPacket {
    pub event: ChannelEvents,
    pub name: String,
    pub channel: String,
    pub channel_name: Option<String>, // Channel display name (for create/delete events)
    pub creator: Option<String>,      // Channel creator (for create/delete events)
    pub timestamp: Option<i64>,       // Unix timestamp in milliseconds
}

impl ChannelEventPacket {
    /// Create a simple join/leave event (legacy format)
    pub fn new(event: ChannelEvents, player_name: String, channel_id: String) -> Self {
        Self {
            event,
            name: player_name,
            channel: channel_id,
            channel_name: None,
            creator: None,
            timestamp: None,
        }
    }

    /// Create a full event with metadata (for create/delete events)
    pub fn new_full(
        event: ChannelEvents,
        player_name: String,
        channel_id: String,
        channel_name: Option<String>,
        creator: Option<String>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        Self {
            event,
            name: player_name,
            channel: channel_id,
            channel_name,
            creator,
            timestamp: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
            ),
        }
    }
}

impl TryFrom<QuicNetworkPacketData> for ChannelEventPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChannelEvent(c) => Ok(c),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum ConnectionEventType {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerPresenceEvent {
    pub player_name: String,
    pub timestamp: i64, // Unix timestamp in milliseconds
    pub event_type: ConnectionEventType,
}

impl TryFrom<QuicNetworkPacketData> for PlayerPresenceEvent {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerPresence(p) => Ok(p),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerErrorType {
    VersionIncompatible {
        client_version: String,
        server_version: String,
    },
}

/// Server Error Packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerErrorPacket {
    pub error_type: ServerErrorType,
    pub message: String,
}

impl TryFrom<QuicNetworkPacketData> for ServerErrorPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ServerError(s) => Ok(s),
            _ => Err(()),
        }
    }
}
