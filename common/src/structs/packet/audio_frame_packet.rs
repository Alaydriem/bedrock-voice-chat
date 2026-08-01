use serde::{Deserialize, Serialize};

use super::audio_frame_metadata::AudioFrameMetadata;
use super::quic_network_packet_data::QuicNetworkPacketData;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    #[serde(with = "serde_bytes")]
    encoded_length: Vec<u8>,

    #[serde(with = "serde_bytes")]
    encoded_timestamp: Vec<u8>,

    pub sample_rate: u32,

    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,

    pub sender: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,

    #[serde(default)]
    pub metadata: Vec<AudioFrameMetadata>,
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
            encoded_length: crate::encoding::Varint::encode(length),
            encoded_timestamp: crate::encoding::Varint::encode(timestamp),
            sample_rate,
            data,
            sender,
            spatial,
            metadata: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: Vec<AudioFrameMetadata>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn length(&self) -> i32 {
        crate::encoding::Varint::decode::<i32>(&self.encoded_length)
            .unwrap_or((0, 0))
            .0
    }

    pub fn timestamp(&self) -> i64 {
        crate::encoding::Varint::decode::<i64>(&self.encoded_timestamp)
            .unwrap_or((0, 0))
            .0
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn encoded_length_size(&self) -> usize {
        self.encoded_length.len()
    }

    pub fn encoded_timestamp_size(&self) -> usize {
        self.encoded_timestamp.len()
    }
}
