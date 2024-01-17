use std::{any::Any, fmt::Debug};

use crate::Coordinate;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// The packet type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    Debug,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuicNetworkPacketData {
    AudioFrame(AudioFramePacket),
    PlayerData(PlayerDataPacket),
    Debug(DebugPacket),
}

/// A network packet to be sent via QUIC
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub author: String,
    pub client_id: Vec<u8>,
    pub data: QuicNetworkPacketData,
}

/// Magic header
pub const QUICK_NETWORK_PACKET_HEADER: &[u8; 5] = &[251, 33, 51, 0, 27];

impl QuicNetworkPacket {
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
            Ok(ds) => match ron::from_str::<QuicNetworkPacket>(&ds) {
                Ok(packet) => return Ok(packet),
                Err(e) => {
                    return Err(anyhow!("{}", e.to_string()));
                }
            },
            Err(e) => {
                return Err(anyhow!(
                    "Unable to deserialize RON packet. Possible packet length issue? {}",
                    e.to_string()
                ));
            }
        }
    }

    /// Whether or not a packet should be broadcasted
    pub fn broadcast(&self) -> bool {
        match self.packet_type {
            PacketType::AudioFrame => false,
            PacketType::Debug => true,
            PacketType::PlayerData => true,
        }
    }

    pub fn get_data(&self) -> QuicNetworkPacketData {
        let data = &self.data as &dyn Any;
        match self.packet_type {
            PacketType::AudioFrame => {
                let ds = data.downcast_ref::<AudioFramePacket>().unwrap();
                return QuicNetworkPacketData::AudioFrame(ds.clone());
            }
            PacketType::Debug => {
                let ds = data.downcast_ref::<DebugPacket>().unwrap();
                return QuicNetworkPacketData::Debug(ds.clone());
            }
            PacketType::PlayerData => {
                let ds = data.downcast_ref::<PlayerDataPacket>().unwrap();
                return QuicNetworkPacketData::PlayerData(ds.clone());
            }
        }
    }
}

/// A single, Opus encoded audio frame
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub author: String,
    pub coordinate: Option<Coordinate>,
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
