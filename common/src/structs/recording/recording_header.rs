use serde::{Deserialize, Serialize};

use super::input_recording_header::InputRecordingHeader;
use super::output_recording_header::OutputRecordingHeader;

/// Union type for all recording headers to enable type-safe decoding
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordingHeader {
    Input(InputRecordingHeader),
    Output(OutputRecordingHeader),
}

impl RecordingHeader {
    /// Decode a header from postcard bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    /// Encode a header to postcard bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Get the sample rate from any header type
    pub fn sample_rate(&self) -> u32 {
        match self {
            RecordingHeader::Input(header) => header.sample_rate,
            RecordingHeader::Output(header) => header.sample_rate,
        }
    }

    /// Get the channel count from any header type
    pub fn channels(&self) -> u16 {
        match self {
            RecordingHeader::Input(header) => header.channels,
            RecordingHeader::Output(header) => header.channels,
        }
    }

    /// Check if this is a spatial audio recording
    pub fn is_spatial(&self) -> bool {
        match self {
            RecordingHeader::Input(_) => false, // Input is never spatial
            RecordingHeader::Output(header) => header.is_spatial,
        }
    }
}
