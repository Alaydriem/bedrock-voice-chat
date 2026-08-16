use log::error;
use rodio::Source;
use std::num::NonZero;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::EncodedAudioFramePacket;
use super::handle::JitterBufferHandle;
use super::source::JitterBufferSource;
use super::source_error::JitterBufferError;
use crate::audio::recording::RecordingProducer;
use crate::diagnostics::PlayerReceiveStats;

pub struct JitterBuffer {
    source: Arc<Mutex<JitterBufferSource>>,
}

impl JitterBuffer {
    /// Create a new JitterBuffer pair with activity detection and recording support
    pub fn create_with_handle_and_activity(
        initial_packet: EncodedAudioFramePacket,
        identifier: String,
        player_name: String,
        activity_tx: Option<flume::Sender<crate::audio::stream::ActivityUpdate>>,
        recording_producer: Option<RecordingProducer>,
        recording_active: Option<Arc<AtomicBool>>,
        receive_stats: Arc<PlayerReceiveStats>,
        transport: common::structs::metrics::TransportKind,
    ) -> Result<(Self, JitterBufferHandle), JitterBufferError> {
        let (tx, rx) = flume::unbounded::<Option<EncodedAudioFramePacket>>();

        let sample_rate = initial_packet.sample_rate as u32;
        let buffer_size_ms = initial_packet.buffer_size_ms as u64;
        let buffer_capacity = ((buffer_size_ms / 20) as usize).max(5); // Minimum 5 frames (20ms each)

        log::info!(
            "[{}] Creating jitter buffer with activity detection for player '{}', capacity: {} frames ({}ms), sample_rate: {}Hz",
            identifier,
            player_name,
            buffer_capacity,
            buffer_size_ms,
            sample_rate
        );

        let source = JitterBufferSource::new_with_activity(
            rx,
            initial_packet,
            buffer_capacity,
            player_name,
            activity_tx,
            recording_producer,
            recording_active,
            receive_stats,
            transport,
        )?;

        let jitter_buffer = Self {
            source: Arc::new(Mutex::new(source)),
        };

        let handle = JitterBufferHandle::new(tx);
        Ok((jitter_buffer, handle))
    }
}

impl Source for JitterBuffer {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        if let Ok(source) = self.source.lock() {
            source.channels()
        } else {
            NonZero::new(1).unwrap()
        }
    }

    fn sample_rate(&self) -> NonZero<u32> {
        if let Ok(source) = self.source.lock() {
            source.sample_rate()
        } else {
            NonZero::new(48000).unwrap()
        }
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for JitterBuffer {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(mut source) = self.source.lock() {
            source.next()
        } else {
            error!("Failed to lock jitter buffer source");
            None
        }
    }
}
