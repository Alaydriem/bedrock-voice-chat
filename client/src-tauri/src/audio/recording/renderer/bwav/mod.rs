use crate::audio::recording::renderer::{AudioRenderer, PcmChunk, PcmStream, SessionInfo};
use async_trait::async_trait;
use bwavfile::{Bext, WaveFmt, WaveWriter};
use chrono::{DateTime, Datelike, Local, TimeZone};
use std::path::Path;

/// BWav audio renderer that outputs PCM BWav files
pub struct BwavRenderer {
    bits_per_sample: u16,
}

impl BwavRenderer {
    /// Create a new BWav renderer with default settings
    pub fn new() -> Self {
        Self {
            // f32 samples
            bits_per_sample: 32,
        }
    }

    /// Create Bext metadata from session info
    fn create_bext(
        &self,
        session_info: &SessionInfo,
        player_name: &str,
        sample_rate: u32,
        first_frame_relative_timestamp_ms: u64,
    ) -> Bext {
        // Convert Unix timestamp (ms) to DateTime, using the actual first packet time
        let actual_start_timestamp = session_info.start_timestamp + first_frame_relative_timestamp_ms;
        let timestamp_secs = actual_start_timestamp / 1000;
        let timestamp_nanos = ((actual_start_timestamp % 1000) * 1_000_000) as u32;
        let dt = DateTime::from_timestamp(timestamp_secs as i64, timestamp_nanos)
            .unwrap_or_else(|| Local::now().to_utc());
        let local_dt: DateTime<Local> = DateTime::from(dt);

        // Calculate time_reference (samples since midnight)
        let midnight = Local
            .with_ymd_and_hms(local_dt.year(), local_dt.month(), local_dt.day(), 0, 0, 0)
            .unwrap();
        let seconds_since_midnight = (local_dt.timestamp() - midnight.timestamp()) as u64;
        let time_reference = seconds_since_midnight * sample_rate as u64;

        Bext {
            description: format!("BVC Recording - {}", player_name),
            originator: "Bedrock Voice Chat".to_string(),
            originator_reference: session_info.session_id.clone(),
            origination_date: local_dt.format("%Y-%m-%d").to_string(),
            origination_time: local_dt.format("%H:%M:%S").to_string(),
            time_reference,
            version: 2,
            umid: None,
            loudness_value: None,
            loudness_range: None,
            max_true_peak_level: None,
            max_momentary_loudness: None,
            max_short_term_loudness: None,
            // https://tech.ebu.ch/docs/tech/tech3285.pdf
            coding_history: format!(
                "A=PCM,F={},W={},M={},T=BVC\r\n",
                sample_rate,
                self.bits_per_sample,
                "mono" // should always be mono?
            ),
        }
    }
}

impl Default for BwavRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioRenderer for BwavRenderer {
    async fn render(
        &mut self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error> {
        let stream = PcmStream::new(session_path, player_name)?;

        let info = stream.info()
            .ok_or_else(|| anyhow::anyhow!("No audio data found for player: {}", player_name))?
            .clone();

        let format = if info.channels == 1 {
            WaveFmt::new_pcm_mono(info.sample_rate, self.bits_per_sample)
        } else {
            WaveFmt::new_pcm_stereo(info.sample_rate, self.bits_per_sample)
        };

        let mut writer = WaveWriter::create(output_path, format)?;

        let bext = self.create_bext(
            &info.session_info,
            player_name,
            info.sample_rate,
            info.first_frame_timestamp_ms,
        );
        writer.write_broadcast_metadata(&bext)?;

        let mut frame_writer = writer.audio_frame_writer()?;
        let mut frames_processed = 0;
        let mut silence_chunks_written = 0;

        // Consume the PCM stream
        for chunk in stream {
            match chunk? {
                PcmChunk::Audio(samples) => {
                    frame_writer.write_frames(&samples)?;
                    frames_processed += 1;
                }
                PcmChunk::Silence(total_samples) => {
                    // Write silence in chunks to avoid huge allocations
                    const MAX_SILENCE_CHUNK_SAMPLES: usize = 48000 * 60;
                    let mut remaining = total_samples;
                    while remaining > 0 {
                        let chunk_size = remaining.min(MAX_SILENCE_CHUNK_SAMPLES);
                        let silence: Vec<f32> = vec![0.0; chunk_size];
                        frame_writer.write_frames(&silence)?;
                        remaining -= chunk_size;
                        silence_chunks_written += 1;
                    }
                }
            }
        }

        log::info!(
            "Rendering {} complete: {} audio frames, {} silence chunks written",
            player_name,
            frames_processed,
            silence_chunks_written
        );

        let _writer = frame_writer.end()?;
        Ok(())
    }

    fn file_extension(&self) -> &str {
        "wav"
    }
}
