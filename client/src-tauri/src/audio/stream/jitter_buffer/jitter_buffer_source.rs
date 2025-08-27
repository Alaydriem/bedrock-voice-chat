use log::error;
use rodio::Source;
use std::collections::VecDeque;
use std::time::Duration;

use super::adaptive::AdaptationEngine;
use super::audio_processor::{AudioProcessor, AudioProcessorError};
use super::metrics::MetricsCollector;
use super::EncodedAudioFramePacket;
use crate::audio::stream::activity_detector::ActivityUpdate;

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

/// Streamlined jitter buffer focused on coordination
pub struct JitterBufferSource {
    // Core components
    audio_processor: AudioProcessor,
    packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,

    // Packet management
    packet_ring: VecDeque<EncodedAudioFramePacket>,

    // Adaptive intelligence
    adaptation_engine: AdaptationEngine,
    metrics_collector: MetricsCollector,

    // Control state
    stopped: bool,
    warmup_packets_received: usize,

    // Timing state (minimal)
    last_output_ts_ms: u64,
    last_accepted_timestamp: u64,

    // Activity detection
    player_name: String,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    last_activity_emission: std::time::Instant,
}

impl JitterBufferSource {
    pub fn new(
        packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
        initial_packet: EncodedAudioFramePacket,
        capacity: usize,
    ) -> Result<Self, JitterBufferError> {
        Self::new_with_activity(
            packet_receiver,
            initial_packet,
            capacity,
            String::new(),
            None,
        )
    }

    pub fn new_with_activity(
        packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
        initial_packet: EncodedAudioFramePacket,
        capacity: usize,
        player_name: String,
        activity_tx: Option<flume::Sender<ActivityUpdate>>,
    ) -> Result<Self, JitterBufferError> {
        let sample_rate = initial_packet.sample_rate as u32;

        // Create audio processor
        let audio_processor = AudioProcessor::new(sample_rate, capacity)?;

        // Create adaptive components
        let adaptation_engine = AdaptationEngine::new(capacity);
        let metrics_collector = MetricsCollector::default();

        // Initialize with first packet
        let mut packet_ring = VecDeque::with_capacity(capacity);
        packet_ring.push_back(initial_packet.clone());

        let mut source = Self {
            audio_processor,
            packet_receiver,
            packet_ring,
            adaptation_engine,
            metrics_collector,
            stopped: false,
            warmup_packets_received: 1, // Count the initial packet
            last_output_ts_ms: initial_packet.timestamp.saturating_sub(20), // FRAME_MS = 20
            last_accepted_timestamp: initial_packet.timestamp,
            player_name,
            activity_tx,
            last_activity_emission: std::time::Instant::now(),
        };

        // Record initial packet
        source
            .metrics_collector
            .record_packet_arrival(initial_packet.timestamp, source.packet_ring.len());

        // Emit initial activity since we have a packet
        source.emit_activity_if_needed();

        Ok(source)
    }

    /// Emit activity update if we have packets and enough time has passed
    fn emit_activity_if_needed(&mut self) {
        // Only emit if we have packets (indicating active audio)
        if self.packet_ring.is_empty() {
            return;
        }

        // Rate limit emissions to every 50ms
        let now = std::time::Instant::now();
        if now.duration_since(self.last_activity_emission).as_millis() < 50 {
            return;
        }

        // Emit activity update
        if let Some(ref tx) = self.activity_tx {
            if !self.player_name.is_empty() {
                let update = ActivityUpdate {
                    player_name: self.player_name.clone(),
                    rms_level: 1.0, // Simple presence indicator (not actual RMS)
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                };

                // Non-blocking send
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

                    // Update accepted timestamp
                    self.last_accepted_timestamp = packet_timestamp;

                    // Check for overflow
                    let current_capacity = self.adaptation_engine.current_capacity();
                    if self.packet_ring.len() >= current_capacity {
                        self.metrics_collector.record_overflow_drop();
                        if !self.packet_ring.is_empty() {
                            self.packet_ring.pop_front(); // Drop oldest packet
                        }
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
                    // Stop signal received
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
            // No packets ready: generate PLC/silence
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
}

impl Iterator for JitterBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Try to get sample from audio processor first
        if let Some(sample) = self.audio_processor.next_sample() {
            // Update output timestamp tracking
            if self.audio_processor.frame_sample_countdown == 0 {
                self.last_output_ts_ms = self.last_output_ts_ms.saturating_add(20);
                // FRAME_MS
            }
            return Some(sample);
        }

        // Drain incoming packets
        self.drain_incoming();

        // Check if stopped and no buffered data
        if self.stopped && self.packet_ring.is_empty() && !self.audio_processor.has_samples() {
            return None;
        }

        // During warmup: return silence until we have enough packets
        let warmup_needed = self.adaptation_engine.warmup_packets_needed();
        if self.warmup_packets_received < warmup_needed {
            return Some(0.0);
        }

        // Process packets or generate PLC
        self.process_next_packet()
    }
}

impl Source for JitterBufferSource {
    fn current_span_len(&self) -> Option<usize> {
        None // Infinite stream
    }

    fn channels(&self) -> u16 {
        1 // Mono
    }

    fn sample_rate(&self) -> u32 {
        self.audio_processor.current_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite stream
    }
}
