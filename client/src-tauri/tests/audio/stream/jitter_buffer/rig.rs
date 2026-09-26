use std::f32::consts::PI;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bvc_client_lib::audio::recording::RawRecordingData;
use bvc_client_lib::diagnostics::PlayerReceiveStats;
use bvc_client_lib::{EncodedAudioFramePacket, JitterBuffer, JitterBufferHandle};
use bytes::Bytes;
use common::RecordingPlayerData;
use common::consts::OPUS_FRAME_DURATION_MS;
use common::structs::metrics::TransportKind;

/// Drives the real jitter buffer with real Opus frames, one sample at a time.
///
/// Every frame is encoded up front from one continuous tone, so the encoder's state is exactly
/// what a speaker's would be. Frame `i` carries timestamp `i * 20`. Frame 0 is the packet the
/// buffer is constructed with and is never sent again. Leaving an index unsent is a lost frame.
pub struct JitterBufferRig {
    buffer: JitterBuffer,
    handle: JitterBufferHandle,
    frames: Vec<Bytes>,
    stats: Arc<PlayerReceiveStats>,
    recordings: flume::Receiver<RawRecordingData>,
}

impl JitterBufferRig {
    pub const SAMPLE_RATE: u32 = 48_000;
    pub const SAMPLES_PER_FRAME: usize = 960;
    pub const LOUD: f32 = 0.5;
    // Below the drain policy's quiet threshold once decoded.
    pub const QUIET: f32 = 0.001;
    const TONE_HZ: f32 = 440.0;
    const MAX_PACKET_BYTES: usize = 4_000;
    // What a buffer runs before `AdaptiveBufferState`'s first adjustment, 500 ms after build.
    const INITIAL_WARMUP: u32 = 3;
    const INITIAL_CAPACITY: u32 = 120;

    pub fn new(frame_count: usize, amplitude: f32, recording: bool) -> Self {
        let frames = Self::encode(frame_count, amplitude);
        let stats = Arc::new(PlayerReceiveStats::new("Alice".to_string()));
        let (recording_tx, recordings) = flume::unbounded();

        let (buffer, handle) = JitterBuffer::for_listener(
            Self::packet(0, frames[0].clone()),
            "Alice".to_string(),
            Some(recording_tx),
            Some(Arc::new(AtomicBool::new(recording))),
            stats.clone(),
            TransportKind::Quic,
        )
        .expect("jitter buffer builds");

        Self {
            buffer,
            handle,
            frames,
            stats,
            recordings,
        }
    }

    /// Fails the test if it ran long enough for the buffer to leave its initial configuration.
    ///
    /// Every assertion about depth, warmup or drain is written against warmup 3 and capacity 120.
    /// Past 500 ms the adaptation engine may move them, and the test would silently be measuring
    /// a different buffer. Call it last in any test whose expectations depend on depth.
    pub fn assert_initial_config(&self) {
        assert_eq!(
            (self.stats.warmup_needed(), self.stats.capacity()),
            (Self::INITIAL_WARMUP, Self::INITIAL_CAPACITY),
            "the buffer adapted mid-test: this run took longer than 500 ms and its depth \
             assertions no longer describe the configuration they were written for"
        );
    }

    fn encode(frame_count: usize, amplitude: f32) -> Vec<Bytes> {
        let mut encoder = opus2::Encoder::new(
            Self::SAMPLE_RATE,
            opus2::Channels::Mono,
            opus2::Application::Voip,
        )
        .expect("opus encoder");

        (0..frame_count)
            .map(|frame| {
                let pcm: Vec<f32> = (0..Self::SAMPLES_PER_FRAME)
                    .map(|n| {
                        let t = (frame * Self::SAMPLES_PER_FRAME + n) as f32
                            / Self::SAMPLE_RATE as f32;
                        amplitude * (2.0 * PI * Self::TONE_HZ * t).sin()
                    })
                    .collect();
                Bytes::from(
                    encoder
                        .encode_vec_float(&pcm, Self::MAX_PACKET_BYTES)
                        .expect("opus encode"),
                )
            })
            .collect()
    }

    fn packet(index: usize, data: Bytes) -> EncodedAudioFramePacket {
        EncodedAudioFramePacket::new(
            index as u64 * OPUS_FRAME_DURATION_MS as u64,
            Self::SAMPLE_RATE,
            data,
            true,
            RecordingPlayerData::unknown(),
            RecordingPlayerData::unknown(),
        )
    }

    pub fn send(&self, index: usize) {
        self.send_payload(index, self.frames[index].clone());
    }

    pub fn send_range(&self, range: Range<usize>) {
        for index in range {
            self.send(index);
        }
    }

    /// Sends frame `index`'s timestamp carrying an arbitrary payload.
    pub fn send_payload(&self, index: usize, payload: Bytes) {
        self.handle
            .enqueue(Self::packet(index, payload))
            .expect("enqueue");
    }

    /// Pulls `frame_count` frames of samples out of the buffer, exactly as rodio would.
    pub fn pull_frames(&mut self, frame_count: usize) -> Vec<f32> {
        (0..frame_count * Self::SAMPLES_PER_FRAME)
            .map(|_| self.buffer.next().unwrap_or(0.0))
            .collect()
    }

    /// Real-time delivery. For each index in `range`, the speaker sends that frame unless it is
    /// in `lost`, and the device plays one frame. A lost frame still costs its 20 ms of playback,
    /// as it would on a real link. Then `flush` more frames are played to empty the buffer.
    ///
    /// Interleaving keeps the backlog at the warmup depth. A test that needs a backlog uses
    /// `send_range` and `pull_frames` directly.
    pub fn stream(&mut self, range: Range<usize>, lost: &[usize], flush: usize) {
        for index in range {
            if !lost.contains(&index) {
                self.send(index);
            }
            self.pull_frames(1);
        }
        self.pull_frames(flush);
    }

    pub fn stats(&self) -> &PlayerReceiveStats {
        &self.stats
    }

    pub fn payload(&self, index: usize) -> Vec<u8> {
        self.frames[index].to_vec()
    }

    /// Every output-recording payload emitted so far, in emission order.
    pub fn recorded_payloads(&self) -> Vec<Vec<u8>> {
        self.recordings
            .try_iter()
            .filter_map(|record| match record {
                RawRecordingData::OutputData { opus_data, .. } => Some(opus_data),
                _ => None,
            })
            .collect()
    }
}
