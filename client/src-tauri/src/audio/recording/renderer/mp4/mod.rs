//! MP4/M4A Opus renderer for lossless audio export
//!
//! This renderer muxes raw Opus packets directly into an MP4 container
//! without re-encoding, preserving original audio quality while achieving
//! ~10x compression compared to BWAV.
//!
//! Features:
//! - Lossless Opus passthrough (no decode/re-encode)
//! - Professional timecode track (tmcd) for NLE compatibility
//! - User data box (udta) with session metadata

pub mod boxes;
pub mod constants;
pub mod timecode;

mod tref_injector;

use tref_injector::TrefInjector;

use timecode::{TimecodeSample, TimecodeTrack, UserDataBox};

use super::{AudioRenderer, OpusChunk, OpusPacketStream, OpusStreamInfo};
use async_trait::async_trait;
use shiguredo_mp4::boxes::{AudioSampleEntryFields, DopsBox, OpusBox, SampleEntry};
use shiguredo_mp4::mux::{Mp4FileMuxer, Sample};
use shiguredo_mp4::{FixedPointNumber, TrackKind};
use std::fs::File;
use std::io::{Seek, Write};
use std::num::{NonZeroU16, NonZeroU32};
use std::path::Path;


/// MP4/M4A renderer with Opus audio
///
/// Creates MP4 files with:
/// - Raw Opus audio track (lossless passthrough)
/// - Timecode track for NLE synchronization
/// - User data with session metadata
pub struct Mp4Renderer;

impl Mp4Renderer {
    /// Create a new MP4 renderer
    pub fn new() -> Self {
        Self
    }

    /// Build OpusBox for sample entry
    fn create_opus_box(&self, info: &OpusStreamInfo) -> OpusBox {
        OpusBox {
            audio: AudioSampleEntryFields {
                data_reference_index: NonZeroU16::new(1).unwrap(),
                channelcount: info.channels,
                samplesize: 16, // Opus uses 16-bit internal representation
                samplerate: FixedPointNumber::new(info.sample_rate as u16, 0u16),
            },
            dops_box: DopsBox {
                output_channel_count: info.channels as u8,
                pre_skip: 312, // Standard Opus encoder delay (6.5ms @ 48kHz)
                input_sample_rate: info.sample_rate,
                output_gain: 0, // No gain adjustment
            },
            unknown_boxes: Vec::new(),
        }
    }

    /// Write an Opus packet to file and register with muxer
    fn write_sample(
        &self,
        file: &mut File,
        muxer: &mut Mp4FileMuxer,
        data: &[u8],
        duration_samples: u32,
        data_offset: &mut u64,
        opus_box: &OpusBox,
        first_sample: &mut bool,
        sample_rate: u32,
    ) -> Result<(), anyhow::Error> {
        // Write Opus data to file
        file.write_all(data)?;

        // Register with muxer
        muxer.append_sample(&Sample {
            track_kind: TrackKind::Audio,
            sample_entry: if *first_sample {
                *first_sample = false;
                Some(SampleEntry::Opus(opus_box.clone()))
            } else {
                None
            },
            keyframe: false,
            timescale: NonZeroU32::new(sample_rate).unwrap(),
            duration: duration_samples,
            data_offset: *data_offset,
            data_size: data.len(),
            composition_time_offset: Some(0),
        })?;

        *data_offset += data.len() as u64;
        Ok(())
    }
}

impl Default for Mp4Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioRenderer for Mp4Renderer {
    async fn render(
        &mut self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error> {
        // Create packet stream for raw Opus passthrough
        let stream = OpusPacketStream::new(session_path, player_name)?;
        let info = stream
            .info()
            .ok_or_else(|| anyhow::anyhow!("No audio data for player: {}", player_name))?
            .clone();

        self.mux(stream, info, output_path)
    }

    fn file_extension(&self) -> &str {
        "m4a"
    }
}

impl Mp4Renderer {
    /// Mux already-encoded Opus packets into the file.
    ///
    /// Separate from `render` because a track can arrive as packets lifted straight out of
    /// the WAL or as packets encoded here from mixed audio, and everything after that point
    /// is the same file being written.
    pub(crate) fn mux<I>(
        &mut self,
        chunks: I,
        info: OpusStreamInfo,
        output_path: &Path,
    ) -> Result<(), anyhow::Error>
    where
        I: IntoIterator<Item = Result<OpusChunk, anyhow::Error>>,
    {
        // Create muxer
        let mut muxer = Mp4FileMuxer::new()?;

        // Create output file
        let mut file = File::create(output_path)?;

        // Write initial boxes (ftyp, free space, mdat header placeholder)
        let initial_bytes = muxer.initial_boxes_bytes();
        file.write_all(initial_bytes)?;

        // Track current file offset for sample data
        let mut data_offset = initial_bytes.len() as u64;

        // Prepare sample entry (only needed for first sample)
        let opus_box = self.create_opus_box(&info);
        let mut first_sample = true;
        let mut total_samples: u64 = 0;

        // Process all packets
        for chunk in chunks {
            match chunk? {
                OpusChunk::Packet {
                    data,
                    duration_samples,
                } => {
                    self.write_sample(
                        &mut file,
                        &mut muxer,
                        &data,
                        duration_samples,
                        &mut data_offset,
                        &opus_box,
                        &mut first_sample,
                        info.sample_rate,
                    )?;
                    total_samples += duration_samples as u64;
                }
                OpusChunk::Silence {
                    data,
                    duration_samples,
                } => {
                    self.write_sample(
                        &mut file,
                        &mut muxer,
                        &data,
                        duration_samples,
                        &mut data_offset,
                        &opus_box,
                        &mut first_sample,
                        info.sample_rate,
                    )?;
                    total_samples += duration_samples as u64;
                }
            }
        }

        // Calculate duration for metadata
        let duration_ms = (total_samples * 1000) / info.sample_rate as u64;

        // Finalize muxer first - this generates the moov box and mdat header
        // DON'T write timecode sample yet - moov will be written at data_offset and would overwrite it
        muxer.finalize()?;
        let finalized = muxer
            .finalized_boxes()
            .ok_or_else(|| anyhow::anyhow!("Muxer not finalized"))?;

        // Create timecode sample using new struct-based API
        let timecode_sample = TimecodeSample::from_stream_info(&info);

        let mut timecode_data_offset = 0u64;

        // Write all boxes from muxer, modifying moov as needed
        for (offset, bytes) in finalized.offset_and_bytes_pairs() {
            // Check if this is the moov box (starts with size + 'moov')
            if bytes.len() >= 8 && &bytes[4..8] == b"moov" {
                // First pass: calculate where timecode data will end up
                let original_content = &bytes[8..]; // Skip size and 'moov' fourcc
                let audio_tref = TrefInjector::create_tref_to_timecode(2);
                let modified_content = TrefInjector::inject_tref_into_audio_trak(original_content, &audio_tref);

                // Create user data box using new struct-based API
                let udta = UserDataBox::from_stream_info(&info, Some(duration_ms));
                let udta_bytes = udta.to_bytes();

                // Timecode data goes AFTER the moov box
                // moov starts at `offset`, new moov size = 8 + modified_content + timecode_track + udta
                // But timecode_track needs to know offset first - chicken and egg!
                // Solution: timecode track size is fixed given the parameters, so calculate it

                // Create a dummy timecode track to get its size
                let dummy_timecode_track = TimecodeTrack::builder()
                    .from_stream_info(&info)
                    .track_id(2)
                    .audio_track_id(1)
                    .duration_samples(total_samples)
                    .data_offset(0)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build timecode track: {}", e))?
                    .to_bytes()
                    .map_err(|e| anyhow::anyhow!("Failed to serialize timecode track: {}", e))?;

                let new_moov_size =
                    8 + modified_content.len() + dummy_timecode_track.len() + udta_bytes.len();

                // Timecode sample data will be written right after moov
                timecode_data_offset = offset + new_moov_size as u64;

                // Now create the real timecode track with correct offset
                let timecode_track = TimecodeTrack::builder()
                    .from_stream_info(&info)
                    .track_id(2)
                    .audio_track_id(1)
                    .duration_samples(total_samples)
                    .data_offset(timecode_data_offset)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build timecode track: {}", e))?
                    .to_bytes()
                    .map_err(|e| anyhow::anyhow!("Failed to serialize timecode track: {}", e))?;

                // Build new moov: modified content (with tref in audio trak) + timecode trak + udta
                let new_size = 8 + modified_content.len() + timecode_track.len() + udta_bytes.len();

                let mut new_moov = Vec::with_capacity(new_size);
                new_moov.extend_from_slice(&(new_size as u32).to_be_bytes());
                new_moov.extend_from_slice(b"moov");
                new_moov.extend_from_slice(&modified_content);
                new_moov.extend_from_slice(&timecode_track);
                new_moov.extend_from_slice(&udta_bytes);

                file.seek(std::io::SeekFrom::Start(offset))?;
                file.write_all(&new_moov)?;
            } else {
                file.seek(std::io::SeekFrom::Start(offset))?;
                file.write_all(bytes)?;
            }
        }

        // Now write timecode sample data AFTER moov
        file.seek(std::io::SeekFrom::Start(timecode_data_offset))?;
        file.write_all(&timecode_sample.to_vec())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp4_renderer_creation() {
        let renderer = Mp4Renderer::new();
        assert_eq!(renderer.file_extension(), "m4a");
    }
}
