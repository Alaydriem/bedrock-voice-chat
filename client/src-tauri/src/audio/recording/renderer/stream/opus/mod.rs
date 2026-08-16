use crate::audio::recording::renderer::WalAudioReader;
use std::path::Path;

mod chunk;
mod silence_encoder;
mod stream_info;

pub use chunk::OpusChunk;
pub use silence_encoder::SilenceEncoder;
pub use stream_info::OpusStreamInfo;

/// Iterator over raw Opus packets from WAL files.
///
/// This iterator:
/// - Yields raw Opus packets for lossless muxing
/// - Detects gaps and generates encoded silence to fill them
pub struct OpusPacketStream {
    reader: WalAudioReader,
    info: Option<OpusStreamInfo>,
    silence_encoder: Option<SilenceEncoder>,
    pending_silence_packets: Vec<(Vec<u8>, u32)>,
    pending_audio_packet: Option<(Vec<u8>, u32)>,
    finished: bool,
    last_timestamp_ms: Option<u64>,
}

impl OpusPacketStream {
    /// Create a new Opus packet stream from a WAL recording session.
    pub fn new(session_path: &Path, player_name: &str) -> Result<Self, anyhow::Error> {
        let reader = WalAudioReader::new(session_path, player_name)?;

        // Peek at first entry to get audio parameters
        let info = if let Some(first_entry) = reader.peek_raw_entry() {
            Some(OpusStreamInfo {
                sample_rate: first_entry.header.sample_rate(),
                channels: first_entry.header.channels(),
                first_packet_timestamp_ms: first_entry.relative_timestamp_ms,
                session_info: reader.session_info().clone(),
            })
        } else {
            None
        };

        // Create silence encoder if we have info
        let silence_encoder = if let Some(ref info) = info {
            Some(SilenceEncoder::new(info.sample_rate, info.channels)?)
        } else {
            None
        };

        Ok(Self {
            reader,
            info,
            silence_encoder,
            pending_silence_packets: Vec::new(),
            pending_audio_packet: None,
            finished: false,
            last_timestamp_ms: None,
        })
    }

    /// Get audio stream metadata.
    pub fn info(&self) -> Option<&OpusStreamInfo> {
        self.info.as_ref()
    }
}

impl Iterator for OpusPacketStream {
    type Item = Result<OpusChunk, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Return pending audio packet if we have one (after silence)
        if let Some((data, duration)) = self.pending_audio_packet.take() {
            return Some(Ok(OpusChunk::Packet {
                data,
                duration_samples: duration,
            }));
        }

        // Return pending silence packets
        if let Some((data, duration)) = self.pending_silence_packets.pop() {
            return Some(Ok(OpusChunk::Silence {
                data,
                duration_samples: duration,
            }));
        }

        // Get next raw entry
        let entry = match self.reader.next_raw_entry() {
            Some(e) => e,
            None => {
                self.finished = true;
                return None;
            }
        };

        let opus_data = entry.opus_data.clone();
        let timestamp_ms = entry.relative_timestamp_ms;
        let sample_rate = self.info.as_ref().map(|i| i.sample_rate).unwrap_or(48000);

        // Fixed 20ms duration (960 samples at 48kHz)
        let duration_samples = (sample_rate / 1000) * 20;

        // Check for gap
        if let Some(last_ts) = self.last_timestamp_ms {
            let expected_next = last_ts + 20;
            if timestamp_ms > expected_next + 30 {
                let gap_ms = timestamp_ms - expected_next;
                log::debug!("Gap detected: {}ms at timestamp {}ms", gap_ms, timestamp_ms);

                // Generate silence packets to fill the gap
                if let Some(ref mut encoder) = self.silence_encoder {
                    let gap_samples = (gap_ms as u32 * sample_rate) / 1000;
                    match encoder.encode_silence(gap_samples) {
                        Ok(silence_packets) => {
                            // Store silence packets (reversed for pop order)
                            self.pending_silence_packets = silence_packets;
                            self.pending_silence_packets.reverse();

                            // Store the actual audio packet for after silence
                            self.pending_audio_packet = Some((opus_data.clone(), duration_samples));

                            // Update timestamp and return first silence packet
                            self.last_timestamp_ms = Some(timestamp_ms);

                            if let Some((data, dur)) = self.pending_silence_packets.pop() {
                                return Some(Ok(OpusChunk::Silence {
                                    data,
                                    duration_samples: dur,
                                }));
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to encode silence: {}", e);
                        }
                    }
                }
            }
        }

        // Update timestamp tracking
        self.last_timestamp_ms = Some(timestamp_ms);

        Some(Ok(OpusChunk::Packet {
            data: opus_data,
            duration_samples,
        }))
    }
}
