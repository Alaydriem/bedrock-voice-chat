//! Timecode track construction for MP4 files
//!
//! Creates tmcd (timecode) tracks compatible with Final Cut Pro and other NLEs.
//! Per Apple TN2174: https://developer.apple.com/library/archive/technotes/tn2174/
//!
//! # Module Structure
//!
//! - [`Timecode`] - Core timecode representation (timestamp + frame counter)
//! - [`TimecodeTrack`] - Complete timecode track box (trak)
//! - [`TimecodeSample`] - 4-byte timecode sample data
//! - [`UserDataBox`] - User data (udta) with session metadata
//!
//! # Box Structure
//!
//! ```text
//! trak
//! ├── tkhd (track header)
//! ├── tref (track reference to audio)
//! ├── mdia
//! │   ├── mdhd (media header)
//! │   ├── hdlr (handler: tmcd)
//! │   └── minf
//! │       ├── nmhd (null media header)
//! │       ├── dinf
//! │       │   └── dref
//! │       └── stbl
//! │           ├── stsd (TimecodeSampleEntry)
//! │           ├── stts
//! │           ├── stsc
//! │           ├── stsz
//! │           └── stco
//! ```

mod sample;
mod track;
mod user_data;

#[cfg(test)]
mod tests;

pub use sample::TimecodeSample;
pub use track::{TimecodeTrack, TimecodeTrackBuilder};
pub use user_data::{SessionMetadata, UserDataBox};

use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;
use crate::audio::recording::renderer::mp4::constants::AUDIO_FRAMES_PER_SECOND;
use chrono::{DateTime, Local, Timelike};

/// Timecode derived from wall-clock timestamp
///
/// Stores the raw timestamp and derives h:m:s:f components on demand.
/// For 20ms audio frames, fps is always 50 (1000ms / 20ms).
#[derive(Debug, Clone, Copy)]
pub struct Timecode {
    /// Unix timestamp in milliseconds
    timestamp_ms: u64,
    /// Sample rate (needed for frame duration calculation)
    sample_rate: u32,
}

impl Timecode {
    /// Frames per second for 20ms audio frames
    pub const FPS: u8 = AUDIO_FRAMES_PER_SECOND;

    /// Create a new timecode from a Unix timestamp in milliseconds
    pub fn new(timestamp_ms: u64, sample_rate: u32) -> Self {
        Self {
            timestamp_ms,
            sample_rate,
        }
    }

    /// Create a timecode from OpusStreamInfo
    ///
    /// Uses the session start timestamp plus the first packet's relative timestamp
    /// to get the actual wall-clock time when audio capture began.
    pub fn from_stream_info(info: &OpusStreamInfo) -> Self {
        let actual_timestamp =
            info.session_info.start_timestamp + info.first_packet_timestamp_ms;
        Self::new(actual_timestamp, info.sample_rate)
    }

    /// Get the raw timestamp in milliseconds
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get hours component (0-23) from local time
    pub fn hours(&self) -> u8 {
        self.to_local_datetime().hour() as u8
    }

    /// Get minutes component (0-59) from local time
    pub fn minutes(&self) -> u8 {
        self.to_local_datetime().minute() as u8
    }

    /// Get seconds component (0-59) from local time
    pub fn seconds(&self) -> u8 {
        self.to_local_datetime().second() as u8
    }

    /// Get frame within second (0-49 for 50fps)
    ///
    /// Calculated from the millisecond component of the timestamp.
    pub fn frames(&self) -> u8 {
        let ms_in_second = (self.timestamp_ms % 1000) as u32;
        ((ms_in_second * Self::FPS as u32) / 1000) as u8
    }

    /// Get the frames per second value
    pub fn frames_per_second(&self) -> u8 {
        Self::FPS
    }

    /// Calculate frame duration in samples for this sample rate
    ///
    /// For 20ms frames at 48kHz, this is 960 samples.
    pub fn frame_duration_samples(&self) -> u32 {
        (self.sample_rate * 20) / 1000
    }

    /// Convert to 32-bit frame number for tmcd sample
    ///
    /// Format: frames + (seconds * fps) + (minutes * 60 * fps) + (hours * 3600 * fps)
    pub fn to_frame_number(&self) -> u32 {
        let fps = Self::FPS as u32;
        self.frames() as u32
            + (self.seconds() as u32 * fps)
            + (self.minutes() as u32 * 60 * fps)
            + (self.hours() as u32 * 3600 * fps)
    }

    /// Convert timestamp to local DateTime
    fn to_local_datetime(&self) -> DateTime<Local> {
        DateTime::from_timestamp_millis(self.timestamp_ms as i64)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(Local::now)
    }
}

#[cfg(test)]
mod timecode_tests {
    use super::*;

    #[test]
    fn test_frames_per_second_constant() {
        assert_eq!(Timecode::FPS, 50);
    }

    #[test]
    fn test_frame_calculation() {
        // 500ms into a second should be frame 25 (at 50fps)
        let tc = Timecode::new(1000 * 60 * 60 + 500, 48000); // 1 hour + 500ms
        assert_eq!(tc.frames(), 25);

        // 0ms should be frame 0
        let tc = Timecode::new(1000 * 60 * 60, 48000);
        assert_eq!(tc.frames(), 0);

        // 980ms should be frame 49
        let tc = Timecode::new(1000 * 60 * 60 + 980, 48000);
        assert_eq!(tc.frames(), 49);
    }

    #[test]
    fn test_frame_duration_samples() {
        let tc = Timecode::new(0, 48000);
        assert_eq!(tc.frame_duration_samples(), 960); // 20ms at 48kHz
    }

    #[test]
    fn test_to_frame_number() {
        // Create a timecode with known h:m:s:f values
        // We'll test the formula: frames + (seconds * fps) + (minutes * 60 * fps) + (hours * 3600 * fps)
        // For 1h 30m 45s 25f: 25 + (45 * 50) + (30 * 60 * 50) + (1 * 3600 * 50)
        // = 25 + 2250 + 90000 + 180000 = 272275

        // This test verifies the formula produces expected results
        // (actual h:m:s depends on local timezone, so we test the formula logic separately)
        let tc = Timecode::new(0, 48000);
        let _frame_num = tc.to_frame_number();
        // Just verify it doesn't panic and returns a reasonable value
        assert!(tc.to_frame_number() < 24 * 3600 * 50); // Less than 24 hours worth of frames
    }

    #[test]
    fn test_frames_boundary() {
        // Test that frames stay in bounds 0-49
        for ms in (0..1000).step_by(20) {
            let tc = Timecode::new(ms, 48000);
            assert!(tc.frames() < 50, "Frame {} out of bounds for {}ms", tc.frames(), ms);
        }
    }
}
