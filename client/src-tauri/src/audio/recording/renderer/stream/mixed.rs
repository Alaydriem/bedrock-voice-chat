use std::path::Path;

use crate::audio::recording::renderer::stream::opus::{OpusChunk, OpusStreamInfo};
use crate::audio::recording::renderer::{DecodedAudioFrame, SessionInfo, TrackMixer, WalAudioReader};

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
}

/// A mixed track, encoded back to Opus so it can be muxed like any other.
pub struct MixedOpusStream {
    chunks: std::vec::IntoIter<OpusChunk>,
    info: OpusStreamInfo,
}

impl MixedOpusStream {
    // What Opus is handed at a time, and the rate the recorder's own silence uses.
    const FRAME_MS: usize = 20;
    const BITRATE: i32 = 64_000;

    pub fn new(session_path: &Path, keys: &[String]) -> Result<Self, anyhow::Error> {
        let track = MixedPcmTrack::new(session_path, keys)?;
        let chunks = Self::encode(&track.samples, track.sample_rate, track.channels)?;

        Ok(Self {
            chunks: chunks.into_iter(),
            info: OpusStreamInfo {
                sample_rate: track.sample_rate,
                channels: track.channels,
                // Where the first sound actually is. Everything downstream turns this into
                // the wall clock the track began at, the same as for a single-key track.
                first_packet_timestamp_ms: track.first_sound_ms,
                session_info: track.session_info,
            },
        })
    }

    pub fn info(&self) -> &OpusStreamInfo {
        &self.info
    }

    fn encode(
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<Vec<OpusChunk>, anyhow::Error> {
        let channels_enum = if channels == 1 {
            opus2::Channels::Mono
        } else {
            opus2::Channels::Stereo
        };
        let mut encoder =
            opus2::Encoder::new(sample_rate, channels_enum, opus2::Application::Audio)?;
        encoder.set_bitrate(opus2::Bitrate::Bits(Self::BITRATE))?;

        let frame_samples = (sample_rate as usize * Self::FRAME_MS) / 1000;
        let block = frame_samples * channels as usize;
        let mut encoded = Vec::new();
        let mut out = vec![0u8; 4000];

        for start in (0..samples.len()).step_by(block) {
            // Opus takes whole frames only, so the tail is padded rather than dropped.
            let mut frame = vec![0.0f32; block];
            let end = (start + block).min(samples.len());
            frame[..end - start].copy_from_slice(&samples[start..end]);

            let len = encoder.encode_float(&frame, &mut out)?;
            encoded.push(OpusChunk::Packet {
                data: out[..len].to_vec(),
                duration_samples: frame_samples as u32,
            });
        }

        Ok(encoded)
    }
}

impl Iterator for MixedOpusStream {
    type Item = Result<OpusChunk, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(Ok)
    }
}
