use crate::{Coordinate, Dimension, Orientation, Player};
use anyhow::{anyhow, Error};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

use super::channel::ChannelEvents;
use async_mutex::Mutex;
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
}

/// A network packet to be sent via QUIC
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub owner: Option<PacketOwner>,
    pub data: QuicNetworkPacketData,
}

/// Magic header
pub const QUICK_NETWORK_PACKET_HEADER: &[u8; 5] = &[251, 33, 51, 0, 27];

impl QuicNetworkPacket {
    /// write_all() returns a stream of data that isn't cleanly deliniated by packets
    /// And flush() may be called at any time.
    /// This takes a reference to an existing `Vec<u8>``, from a receive_stream() and
    /// returns all the QuicNetworkPackets that were sent.
    /// The packet is mutated in place with any partial data
    pub fn from_stream(packet: &mut Vec<u8>) -> Result<Vec<QuicNetworkPacket>, anyhow::Error> {
        let mut packets = Vec::<QuicNetworkPacket>::new();

        // If we didn't get anything we can return immediately
        if packet.is_empty() {
            return Ok(packets);
        }

        loop {
            // Need at least 5 bytes for magic header
            if packet.len() < 5 {
                break;
            }
            
            // The first 5 bytes of the packet should always be the magic header we use to indicate a new packet has started
            // If these bytes don't match the magic header, then rip them off the packet and try again
            // If we don't get any bytes from this then the packet is malformed, and we need more data
            match packet.get(0..5) {
                Some(header) => match header.to_vec().eq(&QUICK_NETWORK_PACKET_HEADER) {
                    true => {}
                    false => {
                        // If the first 5 bytes exist, but they aren't the magic packet header, then we've lost packets
                        // To prevent packet loss from causing a memory leak, we need to advance the pointer in the packet to the position of the next instance of the magic header.

                        match packet
                            .windows(QUICK_NETWORK_PACKET_HEADER.len())
                            .position(|window| window == QUICK_NETWORK_PACKET_HEADER)
                        {
                            Some(position) => {
                                // Reset the packet to the starting point of the magic packet header
                                packet.drain(0..position);
                                // Try to continue
                                continue;
                            }
                            None => {
                                // If this happens we have a bunch of random data without a magic packet.
                                // We should reset the buffer because we can't do anything with this
                                packet.clear();
                                break;
                            }
                        }
                    }
                },
                None => {
                    break;
                }
            }

            // Parse variable-length packet size (starts at byte 5)
            let (payload_len, length_size) = match crate::encoding::decode_varint_u32(&packet[5..]) {
                Ok((len, size)) => (len as usize, size),
                Err(_) => break, // Need more data to parse length
            };

            let header_size = 5 + length_size;  // magic header + varint length
            let total_packet_size = header_size + payload_len;

            if packet.len() >= total_packet_size {
                // We have a complete packet - extract payload before draining
                let payload_data = packet[header_size..total_packet_size].to_vec();
                
                // Remove this packet from buffer
                packet.drain(0..total_packet_size);

                // Parse the payload
                match Self::from_vec(&payload_data) {
                    Ok(p) => packets.push(p),
                    Err(_) => {
                        continue; // Skip malformed packets
                    }
                };
            } else {
                break; // Need more data
            }
        }

        return Ok(packets);
    }

    /// Converts the packet into a parseable string
    pub fn to_vec(&self) -> Result<Vec<u8>, anyhow::Error> {
        // Postcard handles AudioFramePacket efficiently with pre-encoded fields
        let payload = postcard::to_stdvec(&self)?;
        
        let mut buffer = Vec::new();
        buffer.extend_from_slice(QUICK_NETWORK_PACKET_HEADER);
        
        // Use varint encoding for packet length (major space savings)
        buffer.extend(crate::encoding::encode_varint_u32(payload.len() as u32));
        buffer.extend(payload);

        Ok(buffer)
    }

    /// Convers a vec back into a raw packet
    pub fn from_vec(data: &[u8]) -> Result<Self, anyhow::Error> {
        postcard::from_bytes::<QuicNetworkPacket>(data)
            .map_err(|e| anyhow!("Postcard deserialization error: {}", e))
    }

    /// Returns the packet type
    pub fn get_packet_type(&self) -> PacketType {
        return self.packet_type.clone();
    }

    /// Whether or not a packet should be broadcasted
    pub fn is_broadcast(&self) -> bool {
        match self.packet_type {
            PacketType::AudioFrame => false,
            PacketType::Debug => true,
            PacketType::PlayerData => true,
            PacketType::ChannelEvent => true,
            PacketType::Collection => false,
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
            None => String::from("")
        }
    }

    /// Returns the client id
    pub fn get_client_id(&self) -> Vec<u8> {
        match &self.owner {
            Some(owner) => owner.client_id.clone(),
            None => vec![]
        }
    }

    /// Returns the underlying data frame.
    pub fn get_data(&self) -> Option<&QuicNetworkPacketData> {
        Some(&self.data)
    }

    // Updates the coordinates for a given packet with the player position data
    pub async fn update_coordinates(&mut self, player_data: Arc<Cache<String, Player>>) {
        match self.get_packet_type() {
            PacketType::AudioFrame => match self.get_data() {
                Some(data) => {
                    let data = data.to_owned();
                    let data: Result<AudioFramePacket, ()> = data.try_into();

                    match data {
                        Ok(mut data) => match data.coordinate {
                            Some(_) => {}
                            None => match player_data.get(&self.get_author()).await {
                                Some(position) => {
                                    data.coordinate = Some(position.coordinates);
                                    data.dimension = Some(position.dimension);
                                    data.orientation = Some(position.orientation);
                                    let audio_frame: QuicNetworkPacketData =
                                        QuicNetworkPacketData::AudioFrame(data);
                                    self.data = audio_frame;
                                }
                                None => {}
                            },
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

    /// Determines if a given PacketOwner can receive this QuicNetworkPacket
    pub async fn is_receivable(
        &mut self,
        recipient: PacketOwner,
        channel_data: Arc<Mutex<Cache<String, String>>>,
        position_data: Arc<Cache<String, Player>>,
        range: f32,
    ) -> bool {
        match self.get_packet_type() {
            PacketType::AudioFrame => match self.get_data() {
                Some(data) => match data.to_owned().try_into() {
                    Ok(data) => {
                        let mut data: AudioFramePacket = data;

                        // You cannot receive your own audio packets
                        if self.get_author().eq(&recipient.name) {
                            return false;
                        }

                        let player_channels = channel_data.lock_arc().await.clone();
                        let this_player = player_channels.get(&self.get_author()).await;
                        let packet_author = player_channels.get(&recipient.name).await;

                        // If the players are both in the same group, then the audio packet may be received by the sender
                        if this_player.is_some()
                            && packet_author.is_some()
                            && this_player.eq(&packet_author)
                        {
                            // Group audio packets defer to client sending settings, and non-spatial by default
                            if data.spatial.is_none() {
                                data.spatial = Some(false);
                                // Group audio packets are non-spatial
                            }
                            self.data = QuicNetworkPacketData::AudioFrame(data);
                            return true;
                        }

                        // Senders and recipients have different rules, recipiants _must_ exist, whereas senders can be an object or item with arbitrary data
                        let actual_sender = position_data.get(&self.get_author()).await;
                        let actual_recipient = position_data.get(&recipient.name).await;

                        // Determine the sender coordinates and dimension first from the player object, then the packet data
                        let (sender_dimension, sender_coordinates) = match actual_sender {
                            Some(sender) => (sender.dimension, sender.coordinates),
                            None => {
                                let dimension = data.dimension.clone();
                                let coordinates = data.coordinate.clone();
                                if dimension.is_none() || coordinates.is_none() {
                                    return false;
                                }

                                (dimension.unwrap(), coordinates.unwrap())
                            }
                        };

                        match actual_recipient {
                            Some(recipiant) => {
                                // if they aren't in the same dimension, then they can't hear each other
                                if !recipiant.dimension.eq(&sender_dimension) {
                                    return false;
                                }

                                let dx = sender_coordinates.x - recipiant.coordinates.x;
                                let dy = sender_coordinates.y - recipiant.coordinates.y;
                                let dz = sender_coordinates.z - recipiant.coordinates.z;
                                let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                                // Return true of the players are within spatial range of the other player
                                let proximity = 1.73 * range;

                                let spatial = data.spatial.clone();
                                match spatial {
                                    // If the client explicitly wants their audio to be spatial, then calculate the distance and return if they are in range
                                    Some(true) => {
                                        return distance <= proximity;
                                    },
                                    // If the client explicitly set their audio to be broadcast
                                    // Then there is a client implementation issue 
                                    Some(false) => {
                                        return false;
                                    },
                                    // If the client did not set their audio to be spatially received
                                    // Then make it spatial, and return if in range
                                    None => {
                                        data.spatial = Some(true);
                                        self.data = QuicNetworkPacketData::AudioFrame(data);
                                        return distance <= proximity;
                                    }
                                }



                            }
                            None => {
                                return false;
                            },
                        };
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
            // If there are other packet types we want recipiants to receive, this should be updated
            _ => false,
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
    encoded_length: Vec<u8>,    // zigzag+varint encoded i32
    
    #[serde(with = "serde_bytes")] 
    encoded_timestamp: Vec<u8>, // zigzag+varint encoded i64
    
    pub sample_rate: u32,
    
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    
    pub coordinate: Option<Coordinate>,
    pub orientation: Option<Orientation>,
    pub dimension: Option<Dimension>,
    pub spatial: Option<bool>
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
        coordinate: Option<Coordinate>,
        orientation: Option<Orientation>,
        dimension: Option<Dimension>,
        spatial: Option<bool>
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
            coordinate,
            orientation,
            dimension,
            spatial
        }
    }
    
    /// Get the decoded length value
    pub fn length(&self) -> i32 {
        crate::encoding::decode_zigzag_varint_i32(&self.encoded_length)
            .unwrap_or((0, 0)).0
    }
    
    /// Get the decoded timestamp value (Unix timestamp in milliseconds)
    pub fn timestamp(&self) -> i64 {
        crate::encoding::decode_zigzag_varint_i64(&self.encoded_timestamp)
            .unwrap_or((0, 0)).0
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
    pub players: Vec<crate::Player>,
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
pub struct DebugPacket(pub String);

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
