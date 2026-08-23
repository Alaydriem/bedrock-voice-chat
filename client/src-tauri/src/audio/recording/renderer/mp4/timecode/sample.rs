//! Timecode sample data
//!
//! The timecode sample is a 4-byte big-endian frame number that represents
//! the starting timecode of the recording.

use super::Timecode;
use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;

/// A timecode sample containing the frame number
///
/// This is the actual data stored in the mdat section that the timecode
/// track references. It's a single 4-byte big-endian integer representing
/// the frame number at the start of the recording.
#[derive(Debug, Clone, Copy)]
pub struct TimecodeSample {
    timecode: Timecode,
}

impl TimecodeSample {
    /// Create a timecode sample from OpusStreamInfo
    pub fn from_stream_info(info: &OpusStreamInfo) -> Self {
        Self {
            timecode: Timecode::from_stream_info(info),
        }
    }

    /// Create a timecode sample from a Timecode
    pub fn from_timecode(timecode: Timecode) -> Self {
        Self { timecode }
    }

    /// Get the underlying timecode
    pub fn timecode(&self) -> &Timecode {
        &self.timecode
    }

    /// Serialize to 4 bytes (big-endian frame number)
    pub fn to_bytes(&self) -> [u8; 4] {
        self.timecode.to_frame_number().to_be_bytes()
    }

    /// Serialize to a Vec<u8> for compatibility with existing code
    pub fn to_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
