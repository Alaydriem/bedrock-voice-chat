//! Timecode track construction for MP4 files
//!
//! Creates tmcd (timecode) tracks compatible with Final Cut Pro and other NLEs.
//! Per Apple TN2174: https://developer.apple.com/library/archive/technotes/tn2174/
//!
//! Box structure:
//! ```text
//! trak
//! ├── tkhd (track header)
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

use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

/// Timecode information derived from session start
#[derive(Debug, Clone)]
pub struct TimecodeInfo {
    /// Hours (0-23)
    pub hours: u8,
    /// Minutes (0-59)
    pub minutes: u8,
    /// Seconds (0-59)
    pub seconds: u8,
    /// Frames within second (based on pseudo-framerate)
    pub frames: u8,
    /// Total samples since midnight
    pub sample_offset: u64,
    /// Frames per second equivalent (for 20ms audio frames = 50 fps)
    pub frames_per_second: u8,
}

impl TimecodeInfo {
    /// Calculate timecode from relative milliseconds (offset from session start)
    ///
    /// For NLE alignment, all clips from the same session use session start as 00:00:00:00.
    /// Each clip's timecode is its relative offset from that point.
    pub fn from_relative_ms(relative_ms: u64, sample_rate: u32) -> Self {
        // For audio, we treat 20ms frames as "video frames"
        // 48kHz with 960 sample frames = 50 "frames" per second
        let frame_duration_samples = (sample_rate * 20) / 1000; // 960 for 48kHz
        let frames_per_second = (sample_rate / frame_duration_samples) as u8; // 50

        // Convert milliseconds to time components
        let total_seconds = relative_ms / 1000;
        let hours = ((total_seconds / 3600) % 24) as u8;
        let minutes = ((total_seconds / 60) % 60) as u8;
        let seconds = (total_seconds % 60) as u8;

        // Sub-second frames from milliseconds
        let ms_in_second = (relative_ms % 1000) as u32;
        let frames = ((ms_in_second * frames_per_second as u32) / 1000) as u8;

        // Total samples from start
        let sample_offset = (relative_ms * sample_rate as u64) / 1000;

        Self {
            hours,
            minutes,
            seconds,
            frames,
            sample_offset,
            frames_per_second,
        }
    }

    /// Calculate timecode from Unix timestamp (milliseconds) - legacy
    #[allow(dead_code)]
    pub fn from_timestamp(timestamp_ms: u64, sample_rate: u32) -> Self {
        // Convert to local time
        let datetime = DateTime::from_timestamp_millis(timestamp_ms as i64)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(Local::now);

        // For audio, we treat 20ms frames as "video frames"
        let frame_duration_samples = (sample_rate * 20) / 1000;
        let frames_per_second = (sample_rate / frame_duration_samples) as u8;

        let hours = datetime.hour() as u8;
        let minutes = datetime.minute() as u8;
        let seconds = datetime.second() as u8;

        let ms_in_second = (timestamp_ms % 1000) as u32;
        let frames = ((ms_in_second * frames_per_second as u32) / 1000) as u8;

        let midnight = Local
            .with_ymd_and_hms(datetime.year(), datetime.month(), datetime.day(), 0, 0, 0)
            .unwrap();
        let ms_since_midnight = (datetime.timestamp_millis() - midnight.timestamp_millis()) as u64;
        let sample_offset = (ms_since_midnight * sample_rate as u64) / 1000;

        Self {
            hours,
            minutes,
            seconds,
            frames,
            sample_offset,
            frames_per_second,
        }
    }

    /// Convert to 32-bit timecode value for tmcd sample
    /// Format: frames + (seconds * fps) + (minutes * 60 * fps) + (hours * 3600 * fps)
    pub fn to_frame_number(&self) -> u32 {
        let fps = self.frames_per_second as u32;
        self.frames as u32
            + (self.seconds as u32 * fps)
            + (self.minutes as u32 * 60 * fps)
            + (self.hours as u32 * 3600 * fps)
    }
}

/// Write a big-endian u32
fn write_u32_be(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_be_bytes());
}

/// Write a big-endian u16
fn write_u16_be(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_be_bytes());
}

/// Write a big-endian i16
fn write_i16_be(buf: &mut Vec<u8>, val: i16) {
    buf.extend_from_slice(&val.to_be_bytes());
}

/// Write a big-endian u64
fn write_u64_be(buf: &mut Vec<u8>, val: u64) {
    buf.extend_from_slice(&val.to_be_bytes());
}

/// Write a 4-character code
fn write_fourcc(buf: &mut Vec<u8>, code: &[u8; 4]) {
    buf.extend_from_slice(code);
}

/// Write a box with size prefix
fn write_box(buf: &mut Vec<u8>, box_type: &[u8; 4], content: &[u8]) {
    let size = 8 + content.len() as u32;
    write_u32_be(buf, size);
    write_fourcc(buf, box_type);
    buf.extend_from_slice(content);
}

/// Create timecode track for NLE compatibility
///
/// This creates a complete trak box containing a timecode track that
/// encodes the session start time for synchronization in video editors.
///
/// `timecode_data_offset` is the file offset where the 4-byte timecode sample is stored.
pub fn create_timecode_track(
    info: &OpusStreamInfo,
    track_id: u32,
    duration_samples: u64,
    timecode_data_offset: u64,
) -> Result<Vec<u8>, anyhow::Error> {
    let sample_rate = info.sample_rate;
    // Calculate absolute wall-clock time when first packet was captured (matches bwav.rs approach)
    let actual_start_timestamp = info.session_info.start_timestamp + info.first_packet_timestamp_ms;
    let timecode = TimecodeInfo::from_timestamp(actual_start_timestamp, sample_rate);

    let mut trak = Vec::new();

    // === tkhd (Track Header) ===
    let mut tkhd = Vec::new();
    // Version 0, flags 0x000007 (track enabled, in movie, in preview)
    write_u32_be(&mut tkhd, 0x00000007);
    // Creation/modification time (0 = use current)
    write_u32_be(&mut tkhd, 0);
    write_u32_be(&mut tkhd, 0);
    // Track ID
    write_u32_be(&mut tkhd, track_id);
    // Reserved
    write_u32_be(&mut tkhd, 0);
    // Duration (in movie timescale - use sample rate)
    write_u32_be(&mut tkhd, duration_samples as u32);
    // Reserved (8 bytes)
    write_u64_be(&mut tkhd, 0);
    // Layer, alternate group
    write_u16_be(&mut tkhd, 0);
    write_u16_be(&mut tkhd, 0);
    // Volume (0 for timecode track)
    write_u16_be(&mut tkhd, 0);
    // Reserved
    write_u16_be(&mut tkhd, 0);
    // Matrix (identity: 0x00010000 for a, d, w; 0 for others)
    let matrix: [u32; 9] = [
        0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000,
    ];
    for val in matrix {
        write_u32_be(&mut tkhd, val);
    }
    // Width, height (0 for timecode)
    write_u32_be(&mut tkhd, 0);
    write_u32_be(&mut tkhd, 0);

    write_box(&mut trak, b"tkhd", &tkhd);

    // === tref (Track Reference) ===
    // Links this timecode track to the audio track (track ID 1)
    let mut tref = Vec::new();
    let mut tmcd_ref = Vec::new();
    write_u32_be(&mut tmcd_ref, 1); // Reference to audio track (track ID 1)
    write_box(&mut tref, b"tmcd", &tmcd_ref);
    write_box(&mut trak, b"tref", &tref);

    // === mdia (Media) ===
    let mut mdia = Vec::new();

    // --- mdhd (Media Header) ---
    let mut mdhd = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut mdhd, 0);
    // Creation/modification time
    write_u32_be(&mut mdhd, 0);
    write_u32_be(&mut mdhd, 0);
    // Timescale (use audio sample rate for precision)
    write_u32_be(&mut mdhd, sample_rate);
    // Duration
    write_u32_be(&mut mdhd, duration_samples as u32);
    // Language (und = undetermined, packed as 3x5-bit chars)
    // 'u'=21, 'n'=14, 'd'=4 -> 0x55C4
    write_u16_be(&mut mdhd, 0x55C4);
    // Quality
    write_u16_be(&mut mdhd, 0);

    write_box(&mut mdia, b"mdhd", &mdhd);

    // --- hdlr (Handler Reference) ---
    let mut hdlr = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut hdlr, 0);
    // Pre-defined (0 for tmcd)
    write_u32_be(&mut hdlr, 0);
    // Handler type: 'tmcd'
    write_fourcc(&mut hdlr, b"tmcd");
    // Reserved (3 x u32)
    write_u32_be(&mut hdlr, 0);
    write_u32_be(&mut hdlr, 0);
    write_u32_be(&mut hdlr, 0);
    // Name (null-terminated string)
    hdlr.extend_from_slice(b"TimeCodeHandler\0");

    write_box(&mut mdia, b"hdlr", &hdlr);

    // --- minf (Media Information) ---
    let mut minf = Vec::new();

    // ---- nmhd (Null Media Header) ----
    // Per TN2174: Use nmhd instead of gmhd for MP4 timecode tracks
    let mut nmhd = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut nmhd, 0);

    write_box(&mut minf, b"nmhd", &nmhd);

    // ---- dinf (Data Information) ----
    let mut dinf = Vec::new();

    // ----- dref (Data Reference) -----
    let mut dref = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut dref, 0);
    // Entry count
    write_u32_be(&mut dref, 1);
    // URL entry (self-contained)
    let mut url_entry = Vec::new();
    // Version 0, flags 1 (self-contained)
    write_u32_be(&mut url_entry, 0x00000001);
    write_box(&mut dref, b"url ", &url_entry);

    write_box(&mut dinf, b"dref", &dref);
    write_box(&mut minf, b"dinf", &dinf);

    // ---- stbl (Sample Table) ----
    let mut stbl = Vec::new();

    // ----- stsd (Sample Description) -----
    let mut stsd = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut stsd, 0);
    // Entry count
    write_u32_be(&mut stsd, 1);

    // TimecodeSampleEntry ('tmcd')
    let mut tmcd_entry = Vec::new();
    // Reserved (6 bytes)
    tmcd_entry.extend_from_slice(&[0u8; 6]);
    // Data reference index
    write_u16_be(&mut tmcd_entry, 1);
    // Reserved
    write_u32_be(&mut tmcd_entry, 0);
    // Flags: 0x02 = 24-hour wrap
    write_u32_be(&mut tmcd_entry, 0x00000002);
    // Timescale (for frame duration calculation)
    write_u32_be(&mut tmcd_entry, sample_rate);
    // Frame duration (samples per "frame" - 20ms = 960 @ 48kHz)
    let frame_duration = (sample_rate * 20) / 1000;
    write_u32_be(&mut tmcd_entry, frame_duration);
    // Number of frames (pseudo-fps: 50 for 20ms frames)
    tmcd_entry.push(timecode.frames_per_second);
    // Reserved
    tmcd_entry.push(0);

    write_box(&mut stsd, b"tmcd", &tmcd_entry);
    write_box(&mut stbl, b"stsd", &stsd);

    // ----- stts (Time to Sample) -----
    let mut stts = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut stts, 0);
    // Entry count
    write_u32_be(&mut stts, 1);
    // Single entry: all samples have same duration
    write_u32_be(&mut stts, 1); // Sample count
    write_u32_be(&mut stts, duration_samples as u32); // Sample duration

    write_box(&mut stbl, b"stts", &stts);

    // ----- stsc (Sample to Chunk) -----
    let mut stsc = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut stsc, 0);
    // Entry count
    write_u32_be(&mut stsc, 1);
    // Entry: first chunk, samples per chunk, sample description index
    write_u32_be(&mut stsc, 1); // First chunk
    write_u32_be(&mut stsc, 1); // Samples per chunk
    write_u32_be(&mut stsc, 1); // Sample description index

    write_box(&mut stbl, b"stsc", &stsc);

    // ----- stsz (Sample Size) -----
    let mut stsz = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut stsz, 0);
    // Sample size (4 bytes for 32-bit timecode)
    write_u32_be(&mut stsz, 4);
    // Sample count
    write_u32_be(&mut stsz, 1);

    write_box(&mut stbl, b"stsz", &stsz);

    // ----- stco (Chunk Offset) -----
    let mut stco = Vec::new();
    // Version 0, flags 0
    write_u32_be(&mut stco, 0);
    // Entry count
    write_u32_be(&mut stco, 1);
    // Chunk offset - where the 4-byte timecode sample is in the file
    write_u32_be(&mut stco, timecode_data_offset as u32);

    write_box(&mut stbl, b"stco", &stco);

    write_box(&mut minf, b"stbl", &stbl);
    write_box(&mut mdia, b"minf", &minf);
    write_box(&mut trak, b"mdia", &mdia);

    // Wrap in trak box
    let mut result = Vec::new();
    write_box(&mut result, b"trak", &trak);

    Ok(result)
}

/// Create timecode sample data (4-byte frame number)
pub fn create_timecode_sample(info: &OpusStreamInfo) -> Vec<u8> {
    // Calculate absolute wall-clock time when first packet was captured (matches bwav.rs approach)
    let actual_start_timestamp = info.session_info.start_timestamp + info.first_packet_timestamp_ms;
    let timecode = TimecodeInfo::from_timestamp(actual_start_timestamp, info.sample_rate);
    let frame_number = timecode.to_frame_number();

    frame_number.to_be_bytes().to_vec()
}

/// Create user data box with session metadata
///
/// Contains:
/// - Session ID
/// - Start timestamp
/// - Player name
/// - Recording duration
pub fn create_user_data_box(info: &OpusStreamInfo, duration_ms: Option<u64>) -> Vec<u8> {
    let mut udta = Vec::new();

    // Create a meta box with session info
    let mut meta = Vec::new();

    // Version 0, flags 0
    write_u32_be(&mut meta, 0);

    // hdlr for meta
    let mut hdlr = Vec::new();
    write_u32_be(&mut hdlr, 0); // version/flags
    write_u32_be(&mut hdlr, 0); // pre-defined
    write_fourcc(&mut hdlr, b"mdir"); // handler type (metadata)
    write_u32_be(&mut hdlr, 0); // reserved
    write_u32_be(&mut hdlr, 0);
    write_u32_be(&mut hdlr, 0);
    hdlr.extend_from_slice(b"\0"); // empty name

    write_box(&mut meta, b"hdlr", &hdlr);

    // ilst (item list) with custom data
    let mut ilst = Vec::new();

    // Session ID
    write_custom_string_atom(&mut ilst, b"seid", &info.session_info.session_id);

    // Start timestamp
    write_custom_u64_atom(&mut ilst, b"stts", info.session_info.start_timestamp);

    // Player name
    write_custom_string_atom(&mut ilst, b"plyr", &info.session_info.player_name);

    // Duration
    if let Some(duration) = duration_ms {
        write_custom_u64_atom(&mut ilst, b"dura", duration);
    }

    write_box(&mut meta, b"ilst", &ilst);
    write_box(&mut udta, b"meta", &meta);

    // Wrap in udta
    let mut result = Vec::new();
    write_box(&mut result, b"udta", &udta);

    result
}

/// Write a custom string atom
fn write_custom_string_atom(buf: &mut Vec<u8>, name: &[u8; 4], value: &str) {
    let mut atom = Vec::new();

    // data box
    let mut data = Vec::new();
    write_u32_be(&mut data, 1); // type: UTF-8
    write_u32_be(&mut data, 0); // locale
    data.extend_from_slice(value.as_bytes());

    write_box(&mut atom, b"data", &data);
    write_box(buf, name, &atom);
}

/// Write a custom u64 atom
fn write_custom_u64_atom(buf: &mut Vec<u8>, name: &[u8; 4], value: u64) {
    let mut atom = Vec::new();

    // data box
    let mut data = Vec::new();
    write_u32_be(&mut data, 0x15); // type: unsigned int
    write_u32_be(&mut data, 0); // locale
    write_u64_be(&mut data, value);

    write_box(&mut atom, b"data", &data);
    write_box(buf, name, &atom);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timecode_calculation() {
        // Test with a known timestamp: 2024-01-15 14:30:45.500 UTC
        // That's 14:30:45 with 500ms = 25 frames at 50fps
        let ts_ms = 1705329045500u64; // Approximate

        let tc = TimecodeInfo::from_timestamp(ts_ms, 48000);

        // Verify frames_per_second is 50 for 20ms frames
        assert_eq!(tc.frames_per_second, 50);

        // The exact hours/minutes depends on local timezone
        // but frame calculation should be correct
        assert!(tc.frames < 50);
    }

    #[test]
    fn test_frame_number_conversion() {
        let tc = TimecodeInfo {
            hours: 1,
            minutes: 30,
            seconds: 45,
            frames: 25,
            sample_offset: 0,
            frames_per_second: 50,
        };

        // 1*3600*50 + 30*60*50 + 45*50 + 25 = 180000 + 90000 + 2250 + 25 = 272275
        assert_eq!(tc.to_frame_number(), 272275);
    }
}
