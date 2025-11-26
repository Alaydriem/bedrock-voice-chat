mod bwav;
mod pcm_stream;

use async_trait::async_trait;
use common::structs::recording::{RecordingHeader, SessionManifest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use log::debug;
use ts_rs::TS;

pub use bwav::BwavRenderer;
pub use pcm_stream::{PcmChunk, PcmStream, PcmStreamInfo};

/// Audio output format selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub enum AudioFormat {
    Bwav,
}

impl AudioFormat {
    /// Returns the file extension for this format (without dot)
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Bwav => "wav",
        }
    }

    /// Render audio from a session to the specified output path
    pub async fn render(
        &self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error> {
        match self {
            AudioFormat::Bwav => BwavRenderer::new().render(session_path, player_name, output_path).await,
        }
    }
}

/// Trait for rendering audio from WAL recordings to various file formats
#[async_trait]
pub trait AudioRenderer {
    async fn render(
        &mut self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error>;

    fn file_extension(&self) -> &str;
}

/// Decoded audio frame with metadata
#[derive(Debug)]
pub struct DecodedAudioFrame {
    pub pcm_data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub start_timestamp: u64,
    pub player_name: String,
    pub duration_ms: Option<u64>,
}

impl SessionInfo {
    pub fn load(session_path: &Path) -> Result<Self, anyhow::Error> {
        let session_json_path = session_path.join("session.json");
        let manifest: SessionManifest = serde_json::from_str(&std::fs::read_to_string(session_json_path)?)?;

        Ok(Self {
            session_id: manifest.session_id,
            start_timestamp: manifest.start_timestamp,
            player_name: manifest.emitter_player,
            duration_ms: manifest.duration_ms,
        })
    }
}

/// Raw WAL entry containing Opus packet and metadata
#[derive(Debug)]
pub struct WalEntry {
    pub header: RecordingHeader,
    pub opus_data: Vec<u8>,
    pub relative_timestamp_ms: u64,
}

/// WAL audio reader that decodes Opus frames and handles silence gaps
pub struct WalAudioReader {
    entries: Vec<WalEntry>,
    current_index: usize,
    decoder: Option<opus::Decoder>,
    decoder_config: Option<(u32, u16)>,
    session_info: SessionInfo,
}

impl WalAudioReader {
    pub fn new(session_path: &Path, player_name: &str) -> Result<Self, anyhow::Error> {
        let session_info = SessionInfo::load(session_path)?;

        let wal_path = session_path.join("wal");
        let mut entries = Self::read_entries_with_headers(&wal_path, player_name)?;

        entries.sort_by_key(|e| e.relative_timestamp_ms);
        Ok(Self {
            entries,
            current_index: 0,
            decoder: None,
            decoder_config: None,
            session_info,
        })
    }

    /// Get the next raw WAL entry without decoding
    pub fn next_raw_entry(&mut self) -> Option<&WalEntry> {
        if self.current_index >= self.entries.len() {
            return None;
        }

        let entry = &self.entries[self.current_index];
        self.current_index += 1;
        Some(entry)
    }

    /// Peek at the next raw WAL entry without advancing
    pub fn peek_raw_entry(&self) -> Option<&WalEntry> {
        self.entries.get(self.current_index)
    }

    /// Reset the reader to the beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.decoder = None;
        self.decoder_config = None;
    }

    /// Get the next decoded audio frame, inserting silence if needed
    pub fn next_frame(&mut self) -> Result<Option<DecodedAudioFrame>, anyhow::Error> {
        if self.current_index >= self.entries.len() {
            return Ok(None);
        }

        let entry = &self.entries[self.current_index];
        self.current_index += 1;

        let sample_rate = entry.header.sample_rate();
        let channels = entry.header.channels();

        let needs_new_decoder = match self.decoder_config {
            None => true,
            Some((current_rate, current_channels)) => {
                current_rate != sample_rate || current_channels != channels
            }
        };

        if needs_new_decoder {
            self.decoder = Some(opus::Decoder::new(
                sample_rate,
                if channels == 1 { opus::Channels::Mono } else { opus::Channels::Stereo }
            )?);
            self.decoder_config = Some((sample_rate, channels));
        }

        let max_frame_size = ((sample_rate as usize * 120) / 1000) * channels as usize;
        let mut pcm_data = vec![0.0f32; max_frame_size];

        let decoded_samples = self.decoder.as_mut()
            .expect("Decoder should be initialized")
            .decode_float(
                &entry.opus_data,
                &mut pcm_data,
                false
            )?;

        pcm_data.truncate(decoded_samples * channels as usize);

        Ok(Some(DecodedAudioFrame {
            pcm_data,
            sample_rate,
            channels,
            relative_timestamp_ms: entry.relative_timestamp_ms,
        }))
    }

    pub fn calculate_silence_before_next(&self) -> Option<usize> {
        const OPUS_FRAME_MS: u64 = 20;
        const NETWORK_JITTER_TOLERANCE_MS: u64 = 39; // 0-39ms = consecutive packets

        if self.current_index == 0 || self.current_index >= self.entries.len() {
            return None;
        }

        let prev_entry = &self.entries[self.current_index - 1];
        let next_entry = &self.entries[self.current_index];

        let time_gap_ms = next_entry.relative_timestamp_ms
            .saturating_sub(prev_entry.relative_timestamp_ms)
            .saturating_sub(OPUS_FRAME_MS);

        if time_gap_ms <= NETWORK_JITTER_TOLERANCE_MS {
            return None;
        }
        let sample_rate = next_entry.header.sample_rate() as u64;
        let channels = next_entry.header.channels() as usize;

        let silence_samples = match time_gap_ms.checked_mul(sample_rate)
            .and_then(|v| v.checked_div(1000))
            .and_then(|v| v.checked_mul(channels as u64))
        {
            Some(samples) if samples <= usize::MAX as u64 => samples as usize,
            _ => {
                log::error!("Overflow calculating silence: {}ms gap at {}Hz {} channels",
                    time_gap_ms, sample_rate, channels);
                return None;
            }
        };

        Some(silence_samples)
    }

    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    /// Get total number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Read WAL entries with headers by parsing segment files directly
    fn read_entries_with_headers(wal_path: &Path, player_name: &str) -> Result<Vec<WalEntry>, anyhow::Error> {
        const NANO_REC_SIGNATURE: [u8; 6] = *b"NANORC";
        const MAX_HEADER_SIZE: usize = 1024;
        const MAX_CONTENT_SIZE: usize = 50 * 1024;
        const MAX_RECORDS_PER_FILE: usize = 100_000;

        let mut entries = Vec::new();

        // Find all segment files for this player (files are named: PlayerName-hash-sequence.log)
        debug!("Reading directory: {:?}", wal_path);
        let dir_entries = fs::read_dir(wal_path)?;
        let mut segment_files = Vec::new();

        for entry in dir_entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with(player_name) && filename.ends_with(".log") {
                    segment_files.push(entry.path());
                }
            }
        }

        segment_files.sort();

        for file_path in segment_files {
            log::info!("Parsing WAL file: {:?}", file_path);

            let file_bytes = std::fs::read(&file_path)?;
            let mut pos = 0;

            const MAX_HEADER_SEARCH_BYTES: usize = 4096;
            let search_limit = pos + MAX_HEADER_SEARCH_BYTES.min(file_bytes.len().saturating_sub(6));

            while pos + 6 <= search_limit {
                if &file_bytes[pos..pos + 6] == NANO_REC_SIGNATURE {
                    break;
                }
                pos += 1;
            }

            // If signature not found in first 4KB, skip this file
            if pos + 6 > search_limit {
                log::warn!("Could not find NANO_REC_SIGNATURE in {:?}, skipping file", file_path);
                continue;
            }

            let mut records_parsed = 0;
            while pos + 6 <= file_bytes.len() && records_parsed < MAX_RECORDS_PER_FILE {
                if &file_bytes[pos..pos + 6] != NANO_REC_SIGNATURE {
                    break;
                }
                pos += 6;

                // Read header length (2 bytes)
                if pos + 2 > file_bytes.len() {
                    break;
                }
                let header_len = u16::from_le_bytes([file_bytes[pos], file_bytes[pos + 1]]) as usize;
                pos += 2;

                // Validate header length
                if header_len > MAX_HEADER_SIZE {
                    log::warn!("Invalid header_len {} in {:?}, stopping parse", header_len, file_path);
                    break;
                }

                if pos + header_len > file_bytes.len() {
                    break;
                }
                let header_bytes = &file_bytes[pos..pos + header_len];
                pos += header_len;

                if pos + 8 > file_bytes.len() {
                    break;
                }
                let content_len = u64::from_le_bytes([
                    file_bytes[pos], file_bytes[pos + 1], file_bytes[pos + 2], file_bytes[pos + 3],
                    file_bytes[pos + 4], file_bytes[pos + 5], file_bytes[pos + 6], file_bytes[pos + 7],
                ]) as usize;
                pos += 8;

                if content_len > MAX_CONTENT_SIZE {
                    log::warn!("Invalid content_len {} in {:?}, stopping parse", content_len, file_path);
                    break;
                }

                if pos + content_len > file_bytes.len() {
                    break;
                }
                let content = file_bytes[pos..pos + content_len].to_vec();
                pos += content_len;

                if !header_bytes.is_empty() {
                    if let Ok(header) = RecordingHeader::from_bytes(header_bytes) {
                        let relative_timestamp_ms = match &header {
                            RecordingHeader::Input(h) => h.relative_timestamp_ms.unwrap_or(0),
                            RecordingHeader::Output(h) => h.relative_timestamp_ms,
                        };

                        entries.push(WalEntry {
                            header,
                            opus_data: content,
                            relative_timestamp_ms,
                        });
                    }
                }

                records_parsed += 1;
            }

            if records_parsed >= MAX_RECORDS_PER_FILE {
                log::warn!("Hit MAX_RECORDS_PER_FILE limit in {:?}, stopping parse", file_path);
            }

            log::info!("  Parsed {} total records from {:?}", records_parsed, file_path);
        }

        Ok(entries)
    }
}
