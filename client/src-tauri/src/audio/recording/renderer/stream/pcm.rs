use crate::audio::recording::renderer::{SessionInfo, WalAudioReader};
use std::path::Path;

/// Chunk of PCM audio data
#[derive(Debug)]
pub enum PcmChunk {
    /// Decoded audio samples (f32)
    Audio(Vec<f32>),
    /// Silence to fill gaps (sample count)
    Silence(usize),
}

/// Audio metadata from the stream
#[derive(Debug, Clone)]
pub struct PcmStreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub first_frame_timestamp_ms: u64,
    pub session_info: SessionInfo,
}

/// Iterator that yields PCM chunks from WAL files.
///
/// This iterator wraps a `WalAudioReader` and yields:
/// - `PcmChunk::Silence(n)` when there's a gap between frames
/// - `PcmChunk::Audio(samples)` for each decoded audio frame
///
/// The iterator handles Opus decoding internally and provides
/// a format-agnostic stream of PCM data that can be consumed
/// by any audio renderer.
pub struct PcmStream {
    reader: WalAudioReader,
    info: Option<PcmStreamInfo>,
    pending_silence: Option<usize>,
    finished: bool,
}

impl PcmStream {
    /// Create a new PCM stream from a WAL recording session.
    ///
    /// This peeks at the first frame to extract audio metadata
    /// (sample rate, channels, timestamp) which is available via `info()`.
    pub fn new(session_path: &Path, player_name: &str) -> Result<Self, anyhow::Error> {
        let reader = WalAudioReader::new(session_path, player_name)?;

        // Peek at first entry to get audio parameters
        let info = if let Some(first_entry) = reader.peek_raw_entry() {
            Some(PcmStreamInfo {
                sample_rate: first_entry.header.sample_rate(),
                channels: first_entry.header.channels(),
                first_frame_timestamp_ms: first_entry.relative_timestamp_ms,
                session_info: reader.session_info().clone(),
            })
        } else {
            None
        };

        Ok(Self {
            reader,
            info,
            pending_silence: None,
            finished: false,
        })
    }

    /// Get audio stream metadata.
    ///
    /// Returns `None` if there's no audio data in the stream.
    pub fn info(&self) -> Option<&PcmStreamInfo> {
        self.info.as_ref()
    }
}

impl Iterator for PcmStream {
    type Item = Result<PcmChunk, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // If we have pending silence from a previous iteration, emit it first
        if let Some(silence_samples) = self.pending_silence.take() {
            return Some(Ok(PcmChunk::Silence(silence_samples)));
        }

        // Check for silence gap before the next frame
        // This looks at the gap between the previously-read frame and the upcoming one
        if let Some(silence_samples) = self.reader.calculate_silence_before_next() {
            // Store the silence and emit it now
            // The next call will then read the actual audio frame
            self.pending_silence = None; // Will read frame on next call
            return Some(Ok(PcmChunk::Silence(silence_samples)));
        }

        // Get next decoded frame
        match self.reader.next_frame() {
            Ok(Some(frame)) => Some(Ok(PcmChunk::Audio(frame.pcm_data))),
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}
