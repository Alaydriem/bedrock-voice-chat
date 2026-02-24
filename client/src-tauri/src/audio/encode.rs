use anyhow::anyhow;
use audioadapter_buffers::direct::SequentialSlice;
use opus2::{Application, Bitrate, Encoder};
use rodio::Source;
use rubato::audioadapter::Adapter;
use rubato::{Fft, FixedSync, Resampler};
use std::path::Path;

const OPUS_SAMPLE_RATE: u32 = 48000;
const FRAME_SIZE: usize = 960; // 20ms at 48kHz

/// Output from encoding an audio file to Ogg/Opus.
pub struct EncodeOutput {
    pub opus_bytes: Vec<u8>,
    pub duration_ms: u64,
    pub original_filename: String,
}

/// Encodes WAV/MP3/OGG audio files to Ogg/Opus format suitable for server upload.
pub struct AudioFileEncoder;

impl AudioFileEncoder {
    /// Encode an audio file at the given path to Ogg/Opus.
    /// Supports WAV, MP3, FLAC, OGG (via rodio::Decoder).
    pub fn encode(path: &str) -> Result<EncodeOutput, anyhow::Error> {
        let file_path = Path::new(path);
        let original_filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Decode the input file
        let file = std::fs::File::open(file_path)
            .map_err(|e| anyhow!("Failed to open file: {}", e))?;

        let source = rodio::Decoder::new(std::io::BufReader::new(file))
            .map_err(|e| anyhow!("Failed to decode audio file: {}", e))?;

        let source_sample_rate = source.sample_rate().get();
        let source_channels = source.channels().get() as usize;

        // Collect all samples as f32
        let samples: Vec<f32> = source.collect();

        if samples.is_empty() {
            return Err(anyhow!("No audio samples found in file"));
        }

        // Convert to mono if stereo
        let mono_samples: Vec<f32> = if source_channels > 1 {
            samples
                .chunks_exact(source_channels)
                .map(|chunk: &[f32]| chunk.iter().sum::<f32>() / source_channels as f32)
                .collect()
        } else {
            samples
        };

        // Resample to 48kHz if needed
        let resampled = if source_sample_rate != OPUS_SAMPLE_RATE {
            Self::resample(&mono_samples, source_sample_rate)?
        } else {
            mono_samples
        };

        // Create Opus encoder
        let mut encoder =
            Encoder::new(OPUS_SAMPLE_RATE, opus2::Channels::Mono, Application::Audio)
                .map_err(|e| anyhow!("Failed to create Opus encoder: {}", e))?;
        encoder
            .set_bitrate(Bitrate::Bits(64_000))
            .map_err(|e| anyhow!("Failed to set bitrate: {}", e))?;

        // Encode frames and write to Ogg container
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

        let opus_bytes = ogg_writer.finish()?;
        let total_samples = resampled.len() as u64;
        let duration_ms = (total_samples * 1000) / OPUS_SAMPLE_RATE as u64;

        Ok(EncodeOutput {
            opus_bytes,
            duration_ms,
            original_filename,
        })
    }

    /// Resample mono audio from source_rate to 48kHz using rubato.
    fn resample(mono_samples: &[f32], source_rate: u32) -> Result<Vec<f32>, anyhow::Error> {
        let chunk_size = (source_rate / 50) as usize; // 20ms chunks
        let mut resampler = Fft::<f32>::new(
            source_rate as usize,
            OPUS_SAMPLE_RATE as usize,
            chunk_size,
            2, // sub_chunks
            1, // mono
            FixedSync::Input,
        )
        .map_err(|e| anyhow!("Failed to create resampler: {:?}", e))?;

        let mut output = Vec::new();
        let mut input_frames_needed = resampler.input_frames_next();
        let mut pos = 0;

        while pos + input_frames_needed <= mono_samples.len() {
            let chunk = &mono_samples[pos..pos + input_frames_needed];
            let input_adapter =
                SequentialSlice::new(chunk, 1, chunk.len())
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

        Ok(output)
    }
}

/// Minimal Ogg/Opus container writer.
struct OggOpusWriter {
    buffer: Vec<u8>,
    serial: u32,
    page_sequence: u32,
    granule_position: u64,
    sample_rate: u32,
    channels: u8,
}

impl OggOpusWriter {
    fn new(sample_rate: u32, channels: u8) -> Result<Self, anyhow::Error> {
        let serial = rand::random::<u32>();
        let mut writer = Self {
            buffer: Vec::new(),
            serial,
            page_sequence: 0,
            granule_position: 0,
            sample_rate,
            channels,
        };

        writer.write_opus_head()?;
        writer.write_opus_tags()?;
        Ok(writer)
    }

    fn write_opus_head(&mut self) -> Result<(), anyhow::Error> {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"OpusHead");
        packet.push(1); // version
        packet.push(self.channels);
        packet.extend_from_slice(&(0u16).to_le_bytes()); // pre-skip
        packet.extend_from_slice(&self.sample_rate.to_le_bytes());
        packet.extend_from_slice(&(0i16).to_le_bytes()); // output gain
        packet.push(0); // channel mapping family

        self.write_ogg_page(&packet, 0x02, 0)?; // BOS flag
        Ok(())
    }

    fn write_opus_tags(&mut self) -> Result<(), anyhow::Error> {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"OpusTags");
        let vendor = b"BVC";
        packet.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        packet.extend_from_slice(vendor);
        packet.extend_from_slice(&(0u32).to_le_bytes()); // no user comments

        self.write_ogg_page(&packet, 0x00, 0)?;
        Ok(())
    }

    fn write_packet(&mut self, data: &[u8], samples: u64) -> Result<(), anyhow::Error> {
        self.granule_position += samples;
        self.write_ogg_page(data, 0x00, self.granule_position)?;
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<u8>, anyhow::Error> {
        self.write_ogg_page(&[], 0x04, self.granule_position)?;
        Ok(self.buffer)
    }

    fn write_ogg_page(
        &mut self,
        data: &[u8],
        header_type: u8,
        granule: u64,
    ) -> Result<(), anyhow::Error> {
        self.buffer.extend_from_slice(b"OggS");
        self.buffer.push(0); // version
        self.buffer.push(header_type);
        self.buffer.extend_from_slice(&granule.to_le_bytes());
        self.buffer.extend_from_slice(&self.serial.to_le_bytes());
        self.buffer
            .extend_from_slice(&self.page_sequence.to_le_bytes());
        self.page_sequence += 1;

        let crc_offset = self.buffer.len();
        self.buffer.extend_from_slice(&[0u8; 4]); // CRC placeholder

        let segment_count = if data.is_empty() {
            0u8
        } else {
            // Ogg requires a terminating segment < 255 to signal end-of-packet.
            // A 255-byte segment means "continues", so exact multiples of 255
            // need an extra 0-length segment.
            (data.len() / 255 + 1) as u8
        };
        self.buffer.push(segment_count);

        let mut remaining = data.len();
        for _ in 0..segment_count {
            if remaining >= 255 {
                self.buffer.push(255);
                remaining -= 255;
            } else {
                self.buffer.push(remaining as u8);
                remaining = 0;
            }
        }

        self.buffer.extend_from_slice(data);

        let page_start = crc_offset - 22;
        let crc = ogg_crc(&self.buffer[page_start..]);
        self.buffer[crc_offset..crc_offset + 4].copy_from_slice(&crc.to_le_bytes());

        Ok(())
    }
}

/// Ogg CRC-32 calculation.
fn ogg_crc(data: &[u8]) -> u32 {
    static CRC_TABLE: std::sync::LazyLock<[u32; 256]> = std::sync::LazyLock::new(|| {
        let mut table = [0u32; 256];
        for i in 0..256 {
            let mut crc = (i as u32) << 24;
            for _ in 0..8 {
                crc = if crc & 0x80000000 != 0 {
                    (crc << 1) ^ 0x04C11DB7
                } else {
                    crc << 1
                };
            }
            table[i] = crc;
        }
        table
    });

    let mut crc = 0u32;
    for &byte in data {
        let index = ((crc >> 24) ^ byte as u32) as usize;
        crc = (crc << 8) ^ CRC_TABLE[index];
    }
    crc
}
