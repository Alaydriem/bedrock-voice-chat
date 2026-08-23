use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::audio_frame_metadata::AudioFrameMetadata;
use super::quic_network_packet_data::QuicNetworkPacketData;

// `Bytes` rather than `Vec<u8>` on the payload. The server clones a frame's envelope
// once per spatial variant and the ingress path clones it again to attach the sender, so a copy
// of the Opus payload is paid several times per frame; a `Bytes` clone is a refcount increment
// instead. It costs nothing on the wire: `bytes` serializes through `serialize_bytes`, which is
// exactly what `serde_bytes` did for `Vec<u8>`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    // postcard zigzag-varints a signed integer, so this occupies the same bytes a manually
    // encoded varint did without the length prefix a byte-string field also carries.
    pub timestamp: i64,
    pub data: Bytes,
    /// Where the speaker is, attached on a position heartbeat rather than on every frame.
    pub speaker: Option<super::SpeakerPosition>,
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
        speaker: Option<super::SpeakerPosition>,
        spatial: Option<bool>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            timestamp,
            // A move rather than a copy, so callers keep passing an owned `Vec` and pay nothing
            // for the conversion.
            data: Bytes::from(data),
            speaker,
            spatial,
            metadata: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: Vec<AudioFrameMetadata>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}
