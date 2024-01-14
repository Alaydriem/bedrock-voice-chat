use anyhow::anyhow;
use serde::{ Serialize, Deserialize };
use dyn_clone::DynClone;
use std::any::Any;
use crate::Coordinate;

/// A network packet to be sent via QUIC
#[derive(Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub author: String,
    pub client_id: Vec<u8>,
    pub data: Box<dyn PacketTypeTrait>,
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
                tracing::error!("{}", e.to_string());
                return Err(anyhow!("Could not parse packet."));
            }
        }
    }

    /// Convers a vec back into a raw packet
    pub fn from_vec(data: &[u8]) -> Result<Self, anyhow::Error> {
        match std::str::from_utf8(data) {
            Ok(ds) =>
                match ron::from_str::<QuicNetworkPacket>(ds) {
                    Ok(packet) => {
                        return Ok(packet);
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                        return Err(anyhow!("{}", e.to_string()));
                    }
                }
            Err(e) => {
                tracing::error!(
                    "Unable to deserialize RON packet. Possible packet length issue? {}",
                    e.to_string()
                );
                return Err(anyhow!("{}", e.to_string()));
            }
        };
    }
}

/// The packet type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PacketType {
    AudioFrame,
    Positions,
    #[cfg(debug_assertions)]
    Debug,
}

#[typetag::serde]
pub trait PacketTypeTrait: Send + Sync + DynClone {
    fn as_any(&self) -> &dyn Any;
    fn broadcast(&self) -> bool;
}

dyn_clone::clone_trait_object!(PacketTypeTrait);

/// A single, Opus encoded audio frame
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub author: String,
    pub coordinate: Option<Coordinate>,
}

#[typetag::serde]
impl PacketTypeTrait for AudioFramePacket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn broadcast(&self) -> bool {
        return false;
    }
}

/// General Player Positioning data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerDataPacket {
    pub players: Vec<crate::Player>,
}

#[typetag::serde]
impl PacketTypeTrait for PlayerDataPacket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn broadcast(&self) -> bool {
        return true;
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket(pub String);

#[typetag::serde]
impl PacketTypeTrait for DebugPacket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn broadcast(&self) -> bool {
        return false;
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectPacket(pub String);

#[typetag::serde]
impl PacketTypeTrait for ConnectPacket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn broadcast(&self) -> bool {
        return false;
    }
}
