use crate::{ Coordinate, Dimension, Player };
use anyhow::{ anyhow, Error };
use serde::{ Deserialize, Serialize };

use super::channel::ChannelEvents;
use moka::future::Cache;
use std::sync::Arc;
use async_mutex::Mutex;

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
    pub owner: PacketOwner,
    pub data: QuicNetworkPacketData,
}

/// Magic header
pub const QUICK_NETWORK_PACKET_HEADER: &[u8; 5] = &[251, 33, 51, 0, 27];

impl QuicNetworkPacket {
    /// write_all() returns a stream of data that isn't cleanly deliniated by packets
    /// And flush() may be called at any time.
    /// This takes a reference to an existing Vec<u8>, from a receive_stream() and
    /// returns all the QuicNetworkPackets that were sent.
    /// The packet is mutated in place with any partial data
    pub fn from_stream(packet: &mut Vec<u8>) -> Result<Vec<QuicNetworkPacket>, anyhow::Error> {
        let mut packets = Vec::<QuicNetworkPacket>::new();

        // If we didn't get anything we can return immediately
        if packet.len() == 0 {
            return Ok(packets);
        }

        loop {
            // The first 5 bytes of the packet should always be the magic header we use to indicate a new packet has started
            // If these bytes don't match the magic header, then rip them off the packet and try again
            // If we don't get any bytes from this then the packet is malformed, and we need more data
            match packet.get(0..5) {
                Some(header) =>
                    match header.to_vec().eq(&QUICK_NETWORK_PACKET_HEADER) {
                        true => {}
                        false => {
                            // If the first 5 bytes exist, but they aren't the magic packet header, then we've lost packets
                            // To prevent packet loss from causing a memory leak, we need to advance the pointer in the packet to the position of the next instance of the magic header.

                            match
                                packet
                                    .windows(QUICK_NETWORK_PACKET_HEADER.len())
                                    .position(|window| window == QUICK_NETWORK_PACKET_HEADER)
                            {
                                Some(position) => {
                                    // Reset the packet to the starting point of the magic packet header
                                    *packet = packet
                                        .get(position..packet.len())
                                        .unwrap()
                                        .to_vec();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());

                                    // Try to continue
                                    continue;
                                }
                                None => {
                                    // If this happens we have a bunch of random data without a magic packet.
                                    // We should reset the buffer because we can't do anything with this
                                    *packet = Vec::new();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());
                                    break;
                                }
                            }
                        }
                    }
                None => {
                    break;
                }
            }

            // The next 8 bytes should be the packet length
            let length = match packet.get(5..13) {
                Some(bytes) => usize::from_be_bytes(bytes.try_into().unwrap()),
                None => {
                    break;
                }
            };

            if packet.len() >= length + 13 {
                let packet_to_process = packet
                    .get(0..length + 13)
                    .unwrap()
                    .to_vec();

                *packet = packet
                    .get(13 + length..packet.len())
                    .unwrap()
                    .to_vec();

                packet.shrink_to(packet.len());
                packet.truncate(packet.len());

                match Self::from_vec(&packet_to_process[13..]) {
                    Ok(p) => packets.push(p),
                    Err(_) => {
                        continue;
                    }
                };
            } else {
                break;
            }
        }

        return Ok(packets);
    }

    /// Converts the packet into a parseable string
    pub fn to_vec(&self) -> Result<Vec<u8>, anyhow::Error> {
        match ron::to_string(&self) {
            Ok(rs) => {
                let mut header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
                let mut len = rs.as_bytes().len().to_be_bytes().to_vec();
                let mut data = rs.as_bytes().to_vec();

                header.append(&mut len);
                header.append(&mut data);

                return Ok(header);
            }
            Err(e) => {
                return Err(anyhow!("Could not parse packet. {}", e.to_string()));
            }
        }
    }

    /// Convers a vec back into a raw packet
    pub fn from_vec(data: &[u8]) -> Result<Self, anyhow::Error> {
        match std::str::from_utf8(data) {
            Ok(ds) =>
                match ron::from_str::<QuicNetworkPacket>(&ds) {
                    Ok(packet) => {
                        return Ok(packet);
                    }
                    Err(e) => {
                        return Err(anyhow!("{}", e.to_string()));
                    }
                }
            Err(e) => {
                return Err(
                    anyhow!(
                        "Unable to deserialize RON packet. Possible packet length issue? {}",
                        e.to_string()
                    )
                );
            }
        }
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
        return self.owner.name.clone();
    }

    /// Returns the client id
    pub fn get_client_id(&self) -> Vec<u8> {
        return self.owner.client_id.clone();
    }

    /// Returns the underlying data frame.
    pub fn get_data(&self) -> Option<&QuicNetworkPacketData> {
        Some(&self.data)
    }

    // Updates the coordinates for a given packet with the player position data
    pub async fn update_coordinates(&mut self, player_data: Arc<Cache<String, Player>>) {
        match self.get_packet_type() {
            PacketType::AudioFrame =>
                match self.get_data() {
                    Some(data) => {
                        let data = data.to_owned();
                        let data: Result<AudioFramePacket, ()> = data.try_into();

                        match data {
                            Ok(mut data) =>
                                match data.coordinate {
                                    Some(_) => {}
                                    None =>
                                        match player_data.get(&self.get_author()).await {
                                            Some(position) => {
                                                data.coordinate = Some(position.coordinates);
                                                let audio_frame: QuicNetworkPacketData =
                                                    QuicNetworkPacketData::AudioFrame(data);
                                                self.data = audio_frame;
                                            }
                                            None => {}
                                        }
                                }
                            Err(_) => {
                                tracing::error!(
                                    "Could not downcast reference packet to audio frame"
                                );
                            }
                        }
                    }
                    None => {
                        tracing::error!("Could not downcast reference packet to audio frame");
                    }
                }
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
        return match
            ron::from_str::<QuicNetworkPacket>(&String::from_utf8_lossy(message.as_bytes()))
        {
            Ok(packet) => Some(packet),
            Err(e) => {
                tracing::error!("Could not decode QuicNetworkPacket from string {}", e.to_string());
                None
            }
        };
    }

    /// Determines if a given PacketOwner can receive this QuicNetworkPacket
    pub async fn is_receivable(
        &self,
        recipient: PacketOwner,
        channel_data: Arc<Mutex<Cache<String, String>>>,
        position_data: Arc<Cache<String, Player>>,
        range: f32
    ) -> bool {
        match self.get_packet_type() {
            PacketType::AudioFrame =>
                match self.get_data() {
                    Some(data) =>
                        match data.to_owned().try_into() {
                            Ok(data) => {
                                let data: AudioFramePacket = data;

                                if self.get_author().eq(&recipient.name) {
                                    return false;
                                }

                                let player_channels = channel_data.lock_arc().await.clone();
                                let this_player = player_channels.get(&self.get_author()).await;
                                let packet_author = player_channels.get(&recipient.name).await;

                                // If the players are both in the same group, then the audio packet may be received by the sender
                                if
                                    this_player.is_some() &&
                                    packet_author.is_some() &&
                                    this_player.eq(&packet_author)
                                {
                                    return true;
                                }

                                // Senders and recipients have different rules, recipiants _must_ exist, whereas senders can be an object or item with arbitrary data
                                let actual_sender = position_data.get(&self.get_author()).await;
                                let actual_recipient = position_data.get(&recipient.name).await;

                                // Determine the sender coordinates and dimension first from the player object, then the packet data
                                let (sender_dimension, sender_coordinates) = match actual_sender {
                                    Some(sender) => (sender.dimension, sender.coordinates),
                                    None => {
                                        if data.dimension.is_none() || data.coordinate.is_none() {
                                            return false;
                                        }

                                        (data.dimension.unwrap(), data.coordinate.unwrap())
                                    }
                                };

                                return match actual_recipient {
                                    Some(recipiant) => {
                                        // if they aren't in the same dimension, then they can't hear each other
                                        if !recipiant.dimension.eq(&sender_dimension) {
                                            return false;
                                        }

                                        // If they are in the same dimension, then calculate their distance to determine if they are in range
                                        let distance = (
                                            (recipiant.coordinates.x - sender_coordinates.x).powf(
                                                2.0
                                            ) +
                                            (recipiant.coordinates.y - sender_coordinates.y).powf(
                                                2.0
                                            ) +
                                            (recipiant.coordinates.z - sender_coordinates.z).powf(
                                                2.0
                                            )
                                        ).sqrt();

                                        // Return true of the players are within spatial range of the other player
                                        distance <= (3.0_f32).sqrt() * range
                                    }
                                    None => false,
                                };
                            }
                            Err(_) => {
                                tracing::error!("Failed to decode audio frame data");
                                return false;
                            }
                        }
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
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub coordinate: Option<Coordinate>,
    pub dimension: Option<Dimension>,
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
