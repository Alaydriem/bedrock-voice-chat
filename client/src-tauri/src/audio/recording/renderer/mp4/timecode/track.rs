//! Timecode track (trak box) construction
//!
//! Creates the complete timecode track box structure for NLE compatibility.

use super::Timecode;
use crate::audio::recording::renderer::mp4::boxes::BoxWriter;
use crate::audio::recording::renderer::mp4::constants::{
    DREF_SELF_CONTAINED, IDENTITY_MATRIX, LANGUAGE_UNDETERMINED, TKHD_FLAGS_DEFAULT,
    TIMECODE_SAMPLE_SIZE, TMCD_FLAG_24_HOUR_WRAP,
};
use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;

/// Error type for timecode track operations
#[derive(Debug)]
pub enum TimecodeError {
    /// Missing required field in builder
    MissingField(&'static str),
}

impl std::fmt::Display for TimecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimecodeError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for TimecodeError {}

/// A complete timecode track for MP4 files
///
/// Contains all the metadata needed to generate a tmcd track box that links
/// to an audio track for NLE synchronization.
#[derive(Debug, Clone)]
pub struct TimecodeTrack {
    /// Track ID for this timecode track
    track_id: u32,
    /// Track ID of the audio track this timecode references
    audio_track_id: u32,
    /// Sample rate (timescale)
    sample_rate: u32,
    /// Duration in samples
    duration_samples: u64,
    /// File offset where the 4-byte timecode sample will be stored
    data_offset: u64,
    /// Timecode information (start time)
    timecode: Timecode,
}

impl TimecodeTrack {
    /// Create a new builder for constructing a TimecodeTrack
    pub fn builder() -> TimecodeTrackBuilder {
        TimecodeTrackBuilder::new()
    }

    /// Estimated size of the timecode track in bytes
    ///
    /// This is useful for calculating the data offset before building the track.
    /// The actual size may vary slightly but this provides a good estimate.
    pub fn estimated_size() -> usize {
        // trak header (8) + tkhd (~92) + tref (~20) + mdia (variable)
        // Based on actual output: approximately 260-280 bytes
        280
    }

    /// Serialize the timecode track to bytes
    ///
    /// Returns the complete trak box ready to be written to the MP4 file.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TimecodeError> {
        let mut trak = BoxWriter::new();

        // === tkhd (Track Header) ===
        let tkhd = self.build_tkhd();
        trak = trak.write_box(b"tkhd", &tkhd);

        // === tref (Track Reference) ===
        let tref = self.build_tref();
        trak = trak.write_box(b"tref", &tref);

        // === mdia (Media) ===
        let mdia = self.build_mdia();
        trak = trak.write_box(b"mdia", &mdia);

        // Wrap in trak box
        let result = BoxWriter::new()
            .write_box(b"trak", trak.as_bytes())
            .finish();

        Ok(result)
    }

    /// Build the track header box (tkhd)
    fn build_tkhd(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags (enabled, in_movie, in_preview)
            .u32(TKHD_FLAGS_DEFAULT)
            // Creation/modification time (0 = use current)
            .u32(0)
            .u32(0)
            // Track ID
            .u32(self.track_id)
            // Reserved
            .u32(0)
            // Duration (in movie timescale)
            .u32(self.duration_samples as u32)
            // Reserved (8 bytes)
            .u64(0)
            // Layer, alternate group
            .u16(0)
            .u16(0)
            // Volume (0 for timecode track)
            .u16(0)
            // Reserved
            .u16(0)
            // Matrix (identity transformation)
            .u32(IDENTITY_MATRIX[0])
            .u32(IDENTITY_MATRIX[1])
            .u32(IDENTITY_MATRIX[2])
            .u32(IDENTITY_MATRIX[3])
            .u32(IDENTITY_MATRIX[4])
            .u32(IDENTITY_MATRIX[5])
            .u32(IDENTITY_MATRIX[6])
            .u32(IDENTITY_MATRIX[7])
            .u32(IDENTITY_MATRIX[8])
            // Width, height (0 for timecode)
            .u32(0)
            .u32(0)
            .finish()
    }

    /// Build the track reference box (tref)
    fn build_tref(&self) -> Vec<u8> {
        // tmcd reference containing the audio track ID
        let tmcd_ref = BoxWriter::new().u32(self.audio_track_id).finish();

        BoxWriter::new().write_box(b"tmcd", &tmcd_ref).finish()
    }

    /// Build the media box (mdia)
    fn build_mdia(&self) -> Vec<u8> {
        let mut mdia = BoxWriter::new();

        // mdhd (Media Header)
        let mdhd = self.build_mdhd();
        mdia = mdia.write_box(b"mdhd", &mdhd);

        // hdlr (Handler Reference)
        let hdlr = self.build_hdlr();
        mdia = mdia.write_box(b"hdlr", &hdlr);

        // minf (Media Information)
        let minf = self.build_minf();
        mdia = mdia.write_box(b"minf", &minf);

        mdia.finish()
    }

    /// Build the media header box (mdhd)
    fn build_mdhd(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Creation/modification time
            .u32(0)
            .u32(0)
            // Timescale (sample rate)
            .u32(self.sample_rate)
            // Duration
            .u32(self.duration_samples as u32)
            // Language (undetermined)
            .u16(LANGUAGE_UNDETERMINED)
            // Quality
            .u16(0)
            .finish()
    }

    /// Build the handler reference box (hdlr)
    fn build_hdlr(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Pre-defined (0 for tmcd)
            .u32(0)
            // Handler type: 'tmcd'
            .fourcc(b"tmcd")
            // Reserved (3 x u32)
            .u32(0)
            .u32(0)
            .u32(0)
            // Name (null-terminated string)
            .bytes(b"TimeCodeHandler\0")
            .finish()
    }

    /// Build the media information box (minf)
    fn build_minf(&self) -> Vec<u8> {
        let mut minf = BoxWriter::new();

        // nmhd (Null Media Header)
        // Per TN2174: Use nmhd instead of gmhd for MP4 timecode tracks
        let nmhd = BoxWriter::new().u32(0).finish(); // Version 0, flags 0
        minf = minf.write_box(b"nmhd", &nmhd);

        // dinf (Data Information)
        let dinf = self.build_dinf();
        minf = minf.write_box(b"dinf", &dinf);

        // stbl (Sample Table)
        let stbl = self.build_stbl();
        minf = minf.write_box(b"stbl", &stbl);

        minf.finish()
    }

    /// Build the data information box (dinf)
    fn build_dinf(&self) -> Vec<u8> {
        // url entry (self-contained)
        let url_entry = BoxWriter::new().u32(DREF_SELF_CONTAINED).finish();

        // dref (Data Reference)
        let dref = BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Entry count
            .u32(1)
            .write_box(b"url ", &url_entry)
            .finish();

        BoxWriter::new().write_box(b"dref", &dref).finish()
    }

    /// Build the sample table box (stbl)
    fn build_stbl(&self) -> Vec<u8> {
        let mut stbl = BoxWriter::new();

        // stsd (Sample Description)
        let stsd = self.build_stsd();
        stbl = stbl.write_box(b"stsd", &stsd);

        // stts (Time to Sample)
        let stts = self.build_stts();
        stbl = stbl.write_box(b"stts", &stts);

        // stsc (Sample to Chunk)
        let stsc = self.build_stsc();
        stbl = stbl.write_box(b"stsc", &stsc);

        // stsz (Sample Size)
        let stsz = self.build_stsz();
        stbl = stbl.write_box(b"stsz", &stsz);

        // stco (Chunk Offset)
        let stco = self.build_stco();
        stbl = stbl.write_box(b"stco", &stco);

        stbl.finish()
    }

    /// Build the sample description box (stsd)
    fn build_stsd(&self) -> Vec<u8> {
        let frame_duration = self.timecode.frame_duration_samples();

        // TimecodeSampleEntry ('tmcd')
        let tmcd_entry = BoxWriter::new()
            // Reserved (6 bytes)
            .zeros(6)
            // Data reference index
            .u16(1)
            // Reserved
            .u32(0)
            // Flags: 24-hour wrap
            .u32(TMCD_FLAG_24_HOUR_WRAP)
            // Timescale (sample rate)
            .u32(self.sample_rate)
            // Frame duration (samples per "frame")
            .u32(frame_duration)
            // Number of frames (fps)
            .u8(self.timecode.frames_per_second())
            // Reserved
            .u8(0)
            .finish();

        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Entry count
            .u32(1)
            .write_box(b"tmcd", &tmcd_entry)
            .finish()
    }

    /// Build the time to sample box (stts)
    fn build_stts(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Entry count
            .u32(1)
            // Single entry: all samples have same duration
            .u32(1) // Sample count
            .u32(self.duration_samples as u32) // Sample duration
            .finish()
    }

    /// Build the sample to chunk box (stsc)
    fn build_stsc(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Entry count
            .u32(1)
            // Entry: first chunk, samples per chunk, sample description index
            .u32(1) // First chunk
            .u32(1) // Samples per chunk
            .u32(1) // Sample description index
            .finish()
    }

    /// Build the sample size box (stsz)
    fn build_stsz(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Sample size (4 bytes for 32-bit timecode)
            .u32(TIMECODE_SAMPLE_SIZE)
            // Sample count
            .u32(1)
            .finish()
    }

    /// Build the chunk offset box (stco)
    fn build_stco(&self) -> Vec<u8> {
        BoxWriter::new()
            // Version 0, flags 0
            .u32(0)
            // Entry count
            .u32(1)
            // Chunk offset
            .u32(self.data_offset as u32)
            .finish()
    }
}

/// Builder for constructing a TimecodeTrack
#[derive(Debug, Default)]
pub struct TimecodeTrackBuilder {
    track_id: Option<u32>,
    audio_track_id: Option<u32>,
    sample_rate: Option<u32>,
    duration_samples: Option<u64>,
    data_offset: Option<u64>,
    timecode: Option<Timecode>,
}

impl TimecodeTrackBuilder {
    /// Create a new builder with no fields set
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize builder from OpusStreamInfo
    ///
    /// Sets sample_rate and timecode from the stream info.
    pub fn from_stream_info(mut self, info: &OpusStreamInfo) -> Self {
        self.sample_rate = Some(info.sample_rate);
        self.timecode = Some(Timecode::from_stream_info(info));
        self
    }

    /// Set the track ID for this timecode track
    pub fn track_id(mut self, id: u32) -> Self {
        self.track_id = Some(id);
        self
    }

    /// Set the audio track ID that this timecode references
    pub fn audio_track_id(mut self, id: u32) -> Self {
        self.audio_track_id = Some(id);
        self
    }

    /// Set the sample rate (timescale)
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = Some(rate);
        self
    }

    /// Set the duration in samples
    pub fn duration_samples(mut self, samples: u64) -> Self {
        self.duration_samples = Some(samples);
        self
    }

    /// Set the file offset where the timecode sample data will be stored
    pub fn data_offset(mut self, offset: u64) -> Self {
        self.data_offset = Some(offset);
        self
    }

    /// Set the timecode directly
    pub fn timecode(mut self, tc: Timecode) -> Self {
        self.timecode = Some(tc);
        self
    }

    /// Build the TimecodeTrack, returning an error if required fields are missing
    pub fn build(self) -> Result<TimecodeTrack, TimecodeError> {
        Ok(TimecodeTrack {
            track_id: self
                .track_id
                .ok_or(TimecodeError::MissingField("track_id"))?,
            audio_track_id: self
                .audio_track_id
                .ok_or(TimecodeError::MissingField("audio_track_id"))?,
            sample_rate: self
                .sample_rate
                .ok_or(TimecodeError::MissingField("sample_rate"))?,
            duration_samples: self
                .duration_samples
                .ok_or(TimecodeError::MissingField("duration_samples"))?,
            data_offset: self
                .data_offset
                .ok_or(TimecodeError::MissingField("data_offset"))?,
            timecode: self
                .timecode
                .ok_or(TimecodeError::MissingField("timecode"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_missing_field() {
        let result = TimecodeTrackBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_complete() {
        let timecode = Timecode::new(1000, 48000);
        let result = TimecodeTrackBuilder::new()
            .track_id(2)
            .audio_track_id(1)
            .sample_rate(48000)
            .duration_samples(480000)
            .data_offset(1000)
            .timecode(timecode)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_to_bytes_produces_trak() {
        let timecode = Timecode::new(1705329045500, 48000);
        let track = TimecodeTrackBuilder::new()
            .track_id(2)
            .audio_track_id(1)
            .sample_rate(48000)
            .duration_samples(480000)
            .data_offset(1000)
            .timecode(timecode)
            .build()
            .unwrap();

        let bytes = track.to_bytes().unwrap();

        // Verify it starts with a valid box header
        assert!(bytes.len() >= 8);
        // Check fourcc is 'trak'
        assert_eq!(&bytes[4..8], b"trak");
    }
}
