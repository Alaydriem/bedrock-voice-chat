//! MP4 timecode track constants
//!
//! All magic numbers extracted from the MP4/QuickTime specification and Apple TN2174.

/// Opus frame duration in milliseconds (fixed for all sample rates)
pub const OPUS_FRAME_DURATION_MS: u32 = 20;

/// Audio frames per second for 20ms frames (1000ms / 20ms = 50)
pub const AUDIO_FRAMES_PER_SECOND: u8 = 50;

/// Default track header flags: enabled (0x1) + in_movie (0x2) + in_preview (0x4)
pub const TKHD_FLAGS_DEFAULT: u32 = 0x0000_0007;

/// Timecode flag: 24-hour wrap (0x02)
/// Per Apple TN2174, this indicates timecode wraps at 24 hours
pub const TMCD_FLAG_24_HOUR_WRAP: u32 = 0x0000_0002;

/// Language code for "undetermined" in packed ISO 639-2/T format
/// 'u'=21, 'n'=14, 'd'=4 packed as (21-1)<<10 | (14-1)<<5 | (4-1) = 0x55C4
pub const LANGUAGE_UNDETERMINED: u16 = 0x55C4;

/// Identity matrix for track/movie transformation
/// Values are 16.16 fixed-point (0x00010000 = 1.0) and 2.30 fixed-point (0x40000000 = 1.0)
pub const IDENTITY_MATRIX: [u32; 9] = [
    0x0001_0000, // a = 1.0 (scale X)
    0,           // b = 0
    0,           // u = 0
    0,           // c = 0
    0x0001_0000, // d = 1.0 (scale Y)
    0,           // v = 0
    0,           // tx = 0
    0,           // ty = 0
    0x4000_0000, // w = 1.0 (2.30 fixed point)
];

/// Data reference flags: self-contained (data in same file)
pub const DREF_SELF_CONTAINED: u32 = 0x0000_0001;

/// Timecode sample size in bytes (32-bit frame number)
pub const TIMECODE_SAMPLE_SIZE: u32 = 4;

/// Metadata data type: UTF-8 string
pub const METADATA_TYPE_UTF8: u32 = 1;

/// Metadata data type: unsigned integer
pub const METADATA_TYPE_UINT: u32 = 0x15;
