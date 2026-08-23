use log::error;
use rodio::Source;
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::EncodedAudioFramePacket;
use super::adaptive::AdaptationEngine;
use super::audio_processor::AudioProcessor;
use super::metrics::MetricsCollector;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::audio::stream::activity_detector::ActivityUpdate;
use crate::audio::stream::stream_manager::AudioSinkType;
use crate::diagnostics::PlayerReceiveStats;

use super::pending_recording::PendingRecording;
use super::source_error::JitterBufferError;

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
    recording_active: Option<Arc<AtomicBool>>,
    pending_recordings: VecDeque<PendingRecording>,
    current_recording: Option<PendingRecording>,
    // Counters published outward. This source is moved into rodio's graph and no handle to it
    // survives, so anything a diagnostic needs has to be written into shared state created
    // before the move.
    receive_stats: Arc<PlayerReceiveStats>,
    // Consecutive frames served from an empty ring, for telling a gap apart from a pause.
    starved_frames: u32,
}

impl JitterBufferSource {
    // How long an empty ring is still a gap in speech rather than a speaker who has stopped.
    //
    // Frames are 20 ms, so this is 100 ms of nothing arriving. Jitter that the buffer cannot
    // cover sits well inside that; a pause between words, with the noise gate shut and no
    // packets being sent at all, runs past it immediately.
    const CONCEAL_GRACE_FRAMES: u32 = 5;

    pub fn new_with_activity(
        packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
        initial_packet: EncodedAudioFramePacket,
        capacity: usize,
        player_name: String,
        activity_tx: Option<flume::Sender<ActivityUpdate>>,
        recording_producer: Option<RecordingProducer>,
        recording_active: Option<Arc<AtomicBool>>,
        receive_stats: Arc<PlayerReceiveStats>,
        transport: common::structs::metrics::TransportKind,
    ) -> Result<Self, JitterBufferError> {
        let sample_rate = initial_packet.sample_rate as u32;

        let audio_processor = AudioProcessor::new(sample_rate, capacity)?;

        let adaptation_engine = AdaptationEngine::new(capacity, transport);
        let metrics_collector = MetricsCollector::default();

        let mut packet_ring = VecDeque::with_capacity(capacity);
        packet_ring.push_back(initial_packet.clone());

        let mut pending_recordings = VecDeque::new();
        let recording_enabled = recording_active
            .as_ref()
            .map_or(false, |f| f.load(Ordering::SeqCst));
        if recording_enabled && recording_producer.is_some() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            pending_recordings.push_back(PendingRecording {
                opus_data: initial_packet.data.to_vec(),
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
            recording_active,
            pending_recordings,
            current_recording: None,
            receive_stats,
            starved_frames: 0,
        };

        source
            .metrics_collector
            .record_packet_arrival(initial_packet.timestamp, source.packet_ring.len());
        source
            .receive_stats
            .record_arrival(initial_packet.timestamp);
        source.receive_stats.set_ring(
            source.packet_ring.len(),
            source.adaptation_engine.current_capacity(),
            source.adaptation_engine.warmup_packets_needed(),
        );

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
                        self.receive_stats.record_ooo_drop();
                        continue;
                    }

                    self.last_accepted_timestamp = packet_timestamp;

                    let current_capacity = self.adaptation_engine.current_capacity();
                    if self.packet_ring.len() >= current_capacity {
                        self.metrics_collector.record_overflow_drop();
                        self.receive_stats.record_overflow_drop();
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
                    if self
                        .recording_active
                        .as_ref()
                        .map_or(false, |f| f.load(Ordering::SeqCst))
                        && self.recording_producer.is_some()
                    {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        self.pending_recordings.push_back(PendingRecording {
                            opus_data: packet.data.to_vec(),
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
                    self.receive_stats.record_arrival(packet_timestamp);
                    self.receive_stats.set_ring(
                        self.packet_ring.len(),
                        current_capacity,
                        self.adaptation_engine.warmup_packets_needed(),
                    );

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
            self.starved_frames = 0;
            match self.audio_processor.decode_opus(&packet.data) {
                Ok(frames_written) => {
                    self.audio_processor.reset_plc_counter();
                    self.metrics_collector.record_decode_success(frames_written);
                    self.receive_stats.record_decode(frames_written);

                    // Assessment network conditions after successful decode
                    self.adaptation_engine
                        .assess_network_conditions(&self.metrics_collector);

                    self.audio_processor.next_sample()
                }
                Err(e) => {
                    error!("Failed to process packet: {}", e);
                    // A packet arrived and could not be used. Concealment either way.
                    self.generate_plc_sample(true)
                }
            }
        } else {
            // An empty ring when playback needs a frame is the underrun. Recording it here
            // gives `NetworkMetrics::buffer_underruns` its first writer, so
            // `CongestionLevel::from_buffer_metrics` stops reading a constant zero.
            //
            // Its reach is bounded and deliberately so: `AdaptationEngine` is built with a
            // base capacity of 6 frames while `AdaptiveBufferState` clamps to a 60 frame
            // floor, so every reachable multiplier lands on the same clamped target and
            // `adjust_capacity` always declines. Congestion becomes a real value; capacity,
            // warmup, and reorder tolerance do not move. Correcting that clamp belongs to the
            // buffer redesign.
            self.metrics_collector.record_underrun();

            // An empty ring means either a late packet inside speech or a speaker who has
            // stopped talking, and the two must not be reported as the same thing. The noise
            // gate sends nothing between utterances, so a silent speaker starves this buffer on
            // every single frame — which drove "reconstructed" toward 100% for anyone who was
            // mostly listening, and put a "voices will sound rough" verdict on a link that was
            // carrying speech perfectly. Beyond the grace window the speaker is treated as
            // silent, and their silence is not something the network failed to deliver.
            //
            // `metrics_collector` is fed regardless: it is the adaptation engine's input, and
            // congestion should keep seeing every starved frame.
            self.starved_frames = self.starved_frames.saturating_add(1);
            let concealing = self.starved_frames <= Self::CONCEAL_GRACE_FRAMES;
            if concealing {
                self.receive_stats.record_underrun();
            }
            self.generate_plc_sample(concealing)
        }
    }

    /// Generate PLC sample
    ///
    /// `record` is false when the gap is a speaker's pause rather than a delivery failure. The
    /// samples are produced either way — playback needs something to emit — they are just not
    /// counted against the link.
    fn generate_plc_sample(&mut self, record: bool) -> Option<f32> {
        match self.audio_processor.generate_plc() {
            Ok(()) => {
                self.metrics_collector.record_plc_generation();
                if record {
                    self.receive_stats.record_plc();
                }
                self.audio_processor.next_sample()
            }
            Err(_) => {
                self.metrics_collector.record_silence_generation();
                if record {
                    self.receive_stats.record_silence();
                }
                // Fall back to silence
                Some(0.0)
            }
        }
    }

    /// Handle recording sample consumption - emit recording data when a frame is complete
    fn handle_recording_sample_consumed(&mut self) {
        // Only check for producer, NOT the flag
        // If data was queued while recording was active, emit it regardless of current flag state
        // This ensures recordings don't get abandoned when the flag turns off mid-stream
        if self.recording_producer.is_none() {
            return;
        }

        // If no pending recordings, nothing to do
        if self.current_recording.is_none() && self.pending_recordings.is_empty() {
            return;
        }

        // If we don't have a current recording being tracked, try to get one from pending
        if self.current_recording.is_none() {
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
        // Infinite stream
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(self.audio_processor.current_sample_rate).expect("sample rate must be > 0")
    }

    fn total_duration(&self) -> Option<Duration> {
        // Infinite stream
        None
    }
}
