use serde::{ Serialize, Deserialize };
use dyn_clone::DynClone;
use std::any::Any;

/// A network packet to be sent via QUIC
#[derive(Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub author: String,
    pub client_id: Vec<u8>,
    pub data: Box<dyn PacketTypeTrait>,
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
}

dyn_clone::clone_trait_object!(PacketTypeTrait);

/// A single, Opus encoded audio frame
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
}

#[typetag::serde]
impl PacketTypeTrait for AudioFramePacket {
    fn as_any(&self) -> &dyn Any {
        self
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket(pub String);

#[typetag::serde]
impl PacketTypeTrait for DebugPacket {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
