use anyhow::anyhow;
use audioadapter_buffers::direct::SequentialSlice;
use opus2::{Application, Bitrate, Encoder};
use rodio::Source;
use rubato::audioadapter::Adapter;
use rubato::{Fft, FixedSync, Resampler};
use std::path::Path;

use super::ogg_writer::OggOpusWriter;
use super::EncodeOutput;

const OPUS_SAMPLE_RATE: u32 = 48000;
const FRAME_SIZE: usize = 960;

pub struct AudioFileEncoder;

impl AudioFileEncoder {
    pub fn encode(path: &str) -> Result<EncodeOutput, anyhow::Error> {
        let file_path = Path::new(path);
        let original_filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let file =
            std::fs::File::open(file_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;

        let source = rodio::Decoder::new(std::io::BufReader::new(file))
            .map_err(|e| anyhow!("Failed to decode audio file: {}", e))?;

        let source_sample_rate = source.sample_rate().get();
        let source_channels = source.channels().get() as usize;

        let samples: Vec<f32> = source.collect();

        if samples.is_empty() {
            return Err(anyhow!("No audio samples found in file"));
        }

        let mono_samples: Vec<f32> = if source_channels > 1 {
            samples
                .chunks_exact(source_channels)
                .map(|chunk: &[f32]| chunk.iter().sum::<f32>() / source_channels as f32)
                .collect()
        } else {
            samples
        };

        let resampled = if source_sample_rate != OPUS_SAMPLE_RATE {
            Self::resample(&mono_samples, source_sample_rate)?
        } else {
            mono_samples
        };

        let mut encoder = Encoder::new(OPUS_SAMPLE_RATE, opus2::Channels::Mono, Application::Audio)
            .map_err(|e| anyhow!("Failed to create Opus encoder: {}", e))?;
        encoder
            .set_bitrate(Bitrate::Bits(64_000))
            .map_err(|e| anyhow!("Failed to set bitrate: {}", e))?;

        let mut ogg_writer = OggOpusWriter::new(OPUS_SAMPLE_RATE, 1)?;

        let mut pos = 0;
        while pos + FRAME_SIZE <= resampled.len() {
            let frame = &resampled[pos..pos + FRAME_SIZE];
            let encoded = encoder
                .encode_vec_float(frame, FRAME_SIZE * 4)
                .map_err(|e| anyhow!("Opus encode error: {}", e))?;

            if encoded.len() > 3 {
                ogg_writer.write_packet(&encoded, FRAME_SIZE as u64)?;
            }
            pos += FRAME_SIZE;
        }

        if pos < resampled.len() {
            let mut final_frame = vec![0.0f32; FRAME_SIZE];
            final_frame[..resampled.len() - pos].copy_from_slice(&resampled[pos..]);
            let encoded = encoder
                .encode_vec_float(&final_frame, FRAME_SIZE * 4)
                .map_err(|e| anyhow!("Opus encode error on final frame: {}", e))?;

            if encoded.len() > 3 {
                ogg_writer.write_packet(&encoded, FRAME_SIZE as u64)?;
            }
        }

        let opus_bytes = ogg_writer.finish()?;
        let duration_ms = (resampled.len() as u64 * 1000) / OPUS_SAMPLE_RATE as u64;

        Ok(EncodeOutput {
            opus_bytes,
            duration_ms,
            original_filename,
        })
    }

    fn resample(mono_samples: &[f32], source_rate: u32) -> Result<Vec<f32>, anyhow::Error> {
        let chunk_size = (source_rate / 50) as usize;
        let mut resampler = Fft::<f32>::new(
            source_rate as usize,
            OPUS_SAMPLE_RATE as usize,
            chunk_size,
            2,
            1,
            FixedSync::Input,
        )
        .map_err(|e| anyhow!("Failed to create resampler: {:?}", e))?;

        let mut output = Vec::new();
        let mut input_frames_needed = resampler.input_frames_next();
        let mut pos = 0;

        while pos + input_frames_needed <= mono_samples.len() {
            let chunk = &mono_samples[pos..pos + input_frames_needed];
            let input_adapter = SequentialSlice::new(chunk, 1, chunk.len())
                .map_err(|e| anyhow!("Adapter error: {:?}", e))?;

            let resampled = resampler
                .process(&input_adapter, 0, None)
                .map_err(|e| anyhow!("Resample error: {:?}", e))?;

            let frames = resampled.frames();
            let data = resampled.take_data();
            output.extend_from_slice(&data[..frames]);

            pos += input_frames_needed;
            input_frames_needed = resampler.input_frames_next();
        }

        if pos < mono_samples.len() {
            let remaining = &mono_samples[pos..];
            let mut padded = vec![0.0f32; input_frames_needed];
            padded[..remaining.len()].copy_from_slice(remaining);
            let input_adapter = SequentialSlice::new(&padded, 1, padded.len())
                .map_err(|e| anyhow!("Adapter error: {:?}", e))?;

            let resampled = resampler
                .process(&input_adapter, 0, None)
                .map_err(|e| anyhow!("Resample error: {:?}", e))?;

            let expected_output = (remaining.len() as f64
                * OPUS_SAMPLE_RATE as f64
                / source_rate as f64)
                .ceil() as usize;
            let frames = resampled.frames();
            let data = resampled.take_data();
            let actual = expected_output.min(frames);
            output.extend_from_slice(&data[..actual]);
        }

        Ok(output)
    }
}
