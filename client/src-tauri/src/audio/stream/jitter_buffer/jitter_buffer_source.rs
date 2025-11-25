use log::error;
use rodio::Source;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::adaptive::AdaptationEngine;
use super::audio_processor::{AudioProcessor, AudioProcessorError};
use super::metrics::MetricsCollector;
use super::EncodedAudioFramePacket;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::audio::stream::activity_detector::ActivityUpdate;
use crate::audio::stream::stream_manager::AudioSinkType;
use common::PlayerData;

#[derive(Debug)]
pub enum JitterBufferError {
    AudioProcessorError(AudioProcessorError),
    InvalidPacket,
}

impl std::fmt::Display for JitterBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitterBufferError::AudioProcessorError(e) => write!(f, "Audio processor error: {}", e),
            JitterBufferError::InvalidPacket => write!(f, "Invalid packet data"),
        }
    }
}

impl std::error::Error for JitterBufferError {}

impl From<AudioProcessorError> for JitterBufferError {
    fn from(err: AudioProcessorError) -> Self {
        JitterBufferError::AudioProcessorError(err)
    }
}

/// Recording data waiting to be emitted when its corresponding audio samples are consumed
#[derive(Clone)]
struct PendingRecording {
    opus_data: Vec<u8>,
    emitter: PlayerData,
    listener: PlayerData,
    sample_rate: u32,
    is_spatial: bool,
    samples_remaining: usize,
    captured_timestamp_ms: u64,
}

/// Streamlined jitter buffer focused on coordination
pub struct JitterBufferSource {
    audio_processor: AudioProcessor,
    packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
    packet_ring: VecDeque<EncodedAudioFramePacket>,
    adaptation_engine: AdaptationEngine,
    metrics_collector: MetricsCollector,
    stopped: bool,
    warmup_packets_received: usize,
    last_output_ts_ms: u64,
    last_accepted_timestamp: u64,
    player_name: String,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    last_activity_emission: std::time::Instant,
    recording_producer: Option<RecordingProducer>,
    recording_enabled: Arc<AtomicBool>,
    pending_recordings: VecDeque<PendingRecording>,
    current_recording: Option<PendingRecording>,
}

impl JitterBufferSource {
    pub fn new_with_activity(
        packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
        initial_packet: EncodedAudioFramePacket,
        capacity: usize,
        player_name: String,
        activity_tx: Option<flume::Sender<ActivityUpdate>>,
        recording_producer: Option<RecordingProducer>,
        recording_enabled: Arc<AtomicBool>,
    ) -> Result<Self, JitterBufferError> {
        let sample_rate = initial_packet.sample_rate as u32;

        let audio_processor = AudioProcessor::new(sample_rate, capacity)?;

        let adaptation_engine = AdaptationEngine::new(capacity);
        let metrics_collector = MetricsCollector::default();

        let mut packet_ring = VecDeque::with_capacity(capacity);
        packet_ring.push_back(initial_packet.clone());

        let mut pending_recordings = VecDeque::new();
        if recording_enabled.load(Ordering::Relaxed) && recording_producer.is_some() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            pending_recordings.push_back(PendingRecording {
                opus_data: initial_packet.data.clone(),
                emitter: initial_packet.emitter.clone(),
                listener: initial_packet.listener.clone(),
                sample_rate: initial_packet.sample_rate,
                is_spatial: initial_packet.route == AudioSinkType::Spatial,
                samples_remaining: audio_processor.samples_per_frame,
                captured_timestamp_ms: now_ms,
            });
        }

        let mut source = Self {
            audio_processor,
            packet_receiver,
            packet_ring,
            adaptation_engine,
            metrics_collector,
            stopped: false,
            warmup_packets_received: 1,
            last_output_ts_ms: initial_packet.timestamp.saturating_sub(20),
            last_accepted_timestamp: initial_packet.timestamp,
            player_name,
            activity_tx,
            last_activity_emission: std::time::Instant::now(),
            recording_producer,
            recording_enabled,
            pending_recordings,
            current_recording: None,
        };

        source
            .metrics_collector
            .record_packet_arrival(initial_packet.timestamp, source.packet_ring.len());

        source.emit_activity_if_needed();

        Ok(source)
    }

    /// Emit activity update if we have packets and enough time has passed
    fn emit_activity_if_needed(&mut self) {
        if self.packet_ring.is_empty() {
            return;
        }

        // Rate limit emissions to every 50ms
        let now = std::time::Instant::now();
        if now.duration_since(self.last_activity_emission).as_millis() < 50 {
            return;
        }

        if let Some(ref tx) = self.activity_tx {
            if !self.player_name.is_empty() {
                let update = ActivityUpdate {
                    player_name: self.player_name.clone(),
                    rms_level: 1.0,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                };

                let _ = tx.try_send(update);
                self.last_activity_emission = now;
            }
        }
    }

    /// Drain incoming packets from channel
    fn drain_incoming(&mut self) {
        while let Ok(msg) = self.packet_receiver.try_recv() {
            match msg {
                Some(packet) => {
                    let packet_timestamp = packet.timestamp;

                    // Check packet acceptance with adaptive logic
                    if !self.is_packet_acceptable(packet_timestamp) {
                        self.metrics_collector.record_ooo_drop();
                        continue;
                    }

                    self.last_accepted_timestamp = packet_timestamp;

                    let current_capacity = self.adaptation_engine.current_capacity();
                    if self.packet_ring.len() >= current_capacity {
                        self.metrics_collector.record_overflow_drop();
                        if !self.packet_ring.is_empty() {
                            self.packet_ring.pop_front();
                        }

                        if !self.pending_recordings.is_empty() {
                            self.pending_recordings.pop_front();
                        }
                    }

                    // Queue recording data if recording is enabled
                    // Capture timestamp NOW (at packet arrival) - this is the intended playback time
                    // before the jitter buffer adds its delay
                    if self.recording_enabled.load(Ordering::Relaxed) && self.recording_producer.is_some() {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        self.pending_recordings.push_back(PendingRecording {
                            opus_data: packet.data.clone(),
                            emitter: packet.emitter.clone(),
                            listener: packet.listener.clone(),
                            sample_rate: packet.sample_rate,
                            is_spatial: packet.route == AudioSinkType::Spatial,
                            samples_remaining: self.audio_processor.samples_per_frame,
                            captured_timestamp_ms: now_ms,
                        });
                    }

                    // Add packet to ring
                    self.packet_ring.push_back(packet);
                    self.warmup_packets_received = (self.warmup_packets_received + 1)
                        .min(self.adaptation_engine.warmup_packets_needed());

                    // Record metrics
                    self.metrics_collector
                        .record_packet_arrival(packet_timestamp, self.packet_ring.len());
                    self.metrics_collector
                        .update_ring_metrics(self.packet_ring.len());

                    // Emit activity since we just received a packet
                    self.emit_activity_if_needed();
                }
                None => {
                    self.stopped = true;
                }
            }
        }

        // Perform adaptive adjustments
        if let Some(new_capacity) = self
            .adaptation_engine
            .adjust_buffer_if_needed(&self.metrics_collector)
        {
            self.metrics_collector
                .record_adaptation(self.last_accepted_timestamp);

            // Resize packet ring if needed
            if new_capacity < self.packet_ring.len() {
                // Trim excess packets from front
                let excess = self.packet_ring.len() - new_capacity;
                for _ in 0..excess {
                    self.packet_ring.pop_front();
                    // Also trim recording data
                    if !self.pending_recordings.is_empty() {
                        self.pending_recordings.pop_front();
                    }
                }
            }
        }
    }

    /// Check if packet is acceptable with adaptive logic
    fn is_packet_acceptable(&self, packet_timestamp: u64) -> bool {
        // Use adaptive engine for timestamp validation
        self.adaptation_engine
            .is_timestamp_acceptable(packet_timestamp, self.last_accepted_timestamp)
    }

    /// Process next packet from ring
    fn process_next_packet(&mut self) -> Option<f32> {
        if let Some(packet) = self.packet_ring.pop_front() {
            match self.audio_processor.decode_opus(&packet.data) {
                Ok(frames_written) => {
                    self.audio_processor.reset_plc_counter();
                    self.metrics_collector.record_decode_success(frames_written);

                    // Assessment network conditions after successful decode
                    self.adaptation_engine
                        .assess_network_conditions(&self.metrics_collector);

                    self.audio_processor.next_sample()
                }
                Err(e) => {
                    error!("Failed to process packet: {}", e);
                    self.generate_plc_sample()
                }
            }
        } else {
            self.generate_plc_sample()
        }
    }

    /// Generate PLC sample
    fn generate_plc_sample(&mut self) -> Option<f32> {
        match self.audio_processor.generate_plc() {
            Ok(()) => {
                self.metrics_collector.record_plc_generation();
                self.audio_processor.next_sample()
            }
            Err(_) => {
                self.metrics_collector.record_silence_generation();
                Some(0.0) // Fallback to silence
            }
        }
    }

    /// Handle recording sample consumption - emit recording data when a frame is complete
    fn handle_recording_sample_consumed(&mut self) {
        // If no recording producer or not enabled, nothing to do
        if self.recording_producer.is_none() || !self.recording_enabled.load(Ordering::Relaxed) {
            return;
        }

        // If we don't have a current recording being tracked, try to get one from pending
        if self.current_recording.is_none() && !self.pending_recordings.is_empty() {
            self.current_recording = self.pending_recordings.pop_front();
        }

        // Process current recording
        if let Some(ref mut current_rec) = self.current_recording {
            current_rec.samples_remaining = current_rec.samples_remaining.saturating_sub(1);

            // When all samples for this frame have been consumed, emit the recording
            if current_rec.samples_remaining == 0 {
                if let Some(ref producer) = self.recording_producer {
                    // Use the timestamp captured at packet arrival time, not the current time.
                    // This accounts for the jitter buffer delay - the audio was intended to be
                    // heard when the packet arrived, not when it exits the buffer.
                    let recording_data = RawRecordingData::OutputData {
                        absolute_timestamp_ms: Some(current_rec.captured_timestamp_ms),
                        opus_data: current_rec.opus_data.clone(),
                        sample_rate: current_rec.sample_rate,
                        channels: 1,
                        emitter: current_rec.emitter.clone(),
                        listener: current_rec.listener.clone(),
                        is_spatial: current_rec.is_spatial,
                    };

                    let _ = producer.try_send(recording_data);
                }
                self.current_recording = None;
            }
        }
    }
}

impl Iterator for JitterBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.audio_processor.next_sample() {
            if self.audio_processor.frame_sample_countdown == 0 {
                self.last_output_ts_ms = self.last_output_ts_ms.saturating_add(20);
            }

            self.handle_recording_sample_consumed();

            return Some(sample);
        }

        // Drain incoming packets
        self.drain_incoming();

        if self.stopped && self.packet_ring.is_empty() && !self.audio_processor.has_samples() {
            return None;
        }

        // During warmup: return silence until we have enough packets
        let warmup_needed = self.adaptation_engine.warmup_packets_needed();
        if self.warmup_packets_received < warmup_needed {
            return Some(0.0);
        }

        let sample = self.process_next_packet();

        if sample.is_some() {
            self.handle_recording_sample_consumed();
        }

        sample
    }
}

impl Source for JitterBufferSource {
    fn current_span_len(&self) -> Option<usize> {
        None // Infinite stream
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.audio_processor.current_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite stream
    }
}
