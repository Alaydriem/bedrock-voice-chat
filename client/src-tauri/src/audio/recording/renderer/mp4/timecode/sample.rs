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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes_length() {
        let tc = Timecode::new(1705329045500, 48000);
        let sample = TimecodeSample::from_timecode(tc);
        let bytes = sample.to_bytes();
        assert_eq!(bytes.len(), 4);
    }

    #[test]
    fn test_to_vec() {
        let tc = Timecode::new(1705329045500, 48000);
        let sample = TimecodeSample::from_timecode(tc);
        let vec = sample.to_vec();
        let bytes = sample.to_bytes();
        assert_eq!(vec.as_slice(), bytes.as_slice());
    }

    #[test]
    fn test_frame_number_encoding() {
        // Create a timecode and verify the bytes are big-endian
        let tc = Timecode::new(0, 48000);
        let sample = TimecodeSample::from_timecode(tc);
        let frame_num = tc.to_frame_number();
        let bytes = sample.to_bytes();

        // Verify big-endian encoding
        let decoded = u32::from_be_bytes(bytes);
        assert_eq!(decoded, frame_num);
    }
}
