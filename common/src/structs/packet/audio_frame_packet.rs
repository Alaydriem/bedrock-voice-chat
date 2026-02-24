use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;

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
