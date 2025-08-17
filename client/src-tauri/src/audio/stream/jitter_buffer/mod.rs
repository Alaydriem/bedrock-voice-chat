use common::{
    structs::{
        audio::{
            PlayerGainSettings,
            PlayerGainStore
        },
        packet::{AudioFramePacket, PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket}
    }, Coordinate, Orientation, Player
};
use crate::audio::stream::stream_manager::AudioSinkType;
use base64::{engine::general_purpose, Engine as _};

#[derive(Clone, Debug)]
pub(crate) struct DecodedAudioFramePacket {
    pub timestamp: u64,
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub route: AudioSinkType,
    pub coordinate: Option<Coordinate>,
    pub orientation: Option<Orientation>,
    pub owner: Option<PacketOwner>,
}

impl DecodedAudioFramePacket {
    pub fn get_author(&self) -> String {
        match &self.owner {
            Some(owner) => {
                // Utilize the client ID so that the same author can receive and hear multiple incoming
                // network streams. Without this, the audio packets for the same author across two streams
                // come in sequence and playback sounds corrupted
                return general_purpose::STANDARD.encode(&owner.client_id);
            }
            None => String::from("")
        }
    }
}