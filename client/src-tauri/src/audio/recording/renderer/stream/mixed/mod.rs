use std::path::Path;

use crate::audio::recording::renderer::{
    DecodedAudioFrame, SessionInfo, SpatialSource, TrackMixer, WalAudioReader,
};
use crate::audio::spatial::SpatialResolver;

/// Several WAL keys, decoded and summed onto one timeline.
///
/// The jukebox is one thing playing into the world however many sources it draws on, so
/// it leaves as one file. That costs a decode, which is why every track with a single key
/// keeps taking the path that copies its packets straight through.
pub struct MixedPcmTrack {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Where sample zero sits, in milliseconds from the session's start. The wait before
    /// the first sound is stated here rather than encoded into the file.
    pub first_sound_ms: u64,
    pub session_info: SessionInfo,
}

impl MixedPcmTrack {
    pub fn new(session_path: &Path, keys: &[String]) -> Result<Self, anyhow::Error> {
        let mut sources: Vec<Vec<DecodedAudioFrame>> = Vec::new();
        let mut format: Option<(u32, u16)> = None;

        for key in keys {
            let mut reader = WalAudioReader::new(session_path, key)?;
            let mut frames = Vec::new();
            while let Some(frame) = reader.next_frame()? {
                format.get_or_insert((frame.sample_rate, frame.channels));
                frames.push(frame);
            }
            if !frames.is_empty() {
                sources.push(frames);
            }
        }

        let (sample_rate, channels) = format
            .ok_or_else(|| anyhow::anyhow!("no audio behind any of the keys for this track"))?;

        let (samples, first_sound_ms) =
            TrackMixer::mix_from_first_sound(&sources, sample_rate, channels);

        Ok(Self {
            samples,
            sample_rate,
            channels,
            first_sound_ms,
            session_info: SessionInfo::load(session_path)?,
        })
    }

    /// The same timeline, with every source placed where the listener heard it.
    ///
    /// Positioning needs each frame's own header, so this reads the entry and the decoded frame
    /// together rather than going through the plain frame iterator.
    pub fn spatial(
        session_path: &Path,
        keys: &[String],
        resolver: &SpatialResolver,
    ) -> Result<Self, anyhow::Error> {
        let mut sources: Vec<Vec<DecodedAudioFrame>> = Vec::new();
        let mut sample_rate: Option<u32> = None;

        for key in keys {
            let mut reader = WalAudioReader::new(session_path, key)?;
            let mut frames = Vec::new();

            // The header is cloned to end the immutable borrow the peek holds, so the frame can
            // be taken next.
            while let Some(header) = reader.peek_raw_entry().map(|entry| entry.header.clone()) {
                let Some(frame) = reader.next_frame()? else {
                    break;
                };
                sample_rate.get_or_insert(frame.sample_rate);
                frames.push((header, frame));
            }

            if !frames.is_empty() {
                sources.push(SpatialSource::position(frames, resolver));
            }
        }

        let sample_rate = sample_rate
            .ok_or_else(|| anyhow::anyhow!("no audio behind any of the keys for this track"))?;

        // Positioned audio is always two channels, whatever the recorder captured.
        let channels = 2;
        let (samples, first_sound_ms) =
            TrackMixer::mix_from_first_sound(&sources, sample_rate, channels);

        Ok(Self {
            samples,
            sample_rate,
            channels,
            first_sound_ms,
            session_info: SessionInfo::load(session_path)?,
        })
    }
}

mod opus_stream;

pub use opus_stream::MixedOpusStream;
