use log::error;
use rodio::Source;
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::EncodedAudioFramePacket;
use super::adaptive::{AdaptationEngine, DrainPolicy};
use super::admission::{Admission, FrameAdmission};
use super::warmup_gate::WarmupGate;
use super::audio_processor::AudioProcessor;
use super::metrics::MetricsCollector;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::audio::stream::activity_detector::ActivityUpdate;
use crate::audio::stream::stream_manager::AudioSinkType;
use crate::diagnostics::PlayerReceiveStats;

use super::ring_entry::RingEntry;
use super::source_error::JitterBufferError;

/// Streamlined jitter buffer focused on coordination
pub struct JitterBufferSource {
    audio_processor: AudioProcessor,
    packet_receiver: flume::Receiver<Option<EncodedAudioFramePacket>>,
    packet_ring: VecDeque<RingEntry>,
    adaptation_engine: AdaptationEngine,
    drain: DrainPolicy,
    metrics_collector: MetricsCollector,
    stopped: bool,
    warmup: WarmupGate,
    admission: FrameAdmission,
    player_name: String,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    last_activity_emission: std::time::Instant,
    recording_producer: Option<RecordingProducer>,
    recording_active: Option<Arc<AtomicBool>>,
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

        let recording_enabled = recording_active
            .as_ref()
            .map_or(false, |f| f.load(Ordering::SeqCst));
        let recorded_at_ms = (recording_enabled && recording_producer.is_some())
            .then(Self::now_ms);

        let mut packet_ring = VecDeque::with_capacity(capacity);
        packet_ring.push_back(RingEntry {
            packet: initial_packet.clone(),
            recorded_at_ms,
        });

        let mut source = Self {
            audio_processor,
            packet_receiver,
            packet_ring,
            adaptation_engine,
            drain: DrainPolicy::new(),
            metrics_collector,
            stopped: false,
            warmup: WarmupGate::new(),
            admission: FrameAdmission::new(initial_packet.timestamp),
            player_name,
            activity_tx,
            last_activity_emission: std::time::Instant::now(),
            recording_producer,
            recording_active,
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

                    match self.admission.admit(packet_timestamp) {
                        Admission::Accept { missing_frames } => {
                            self.receive_stats.record_gap(missing_frames)
                        }
                        Admission::Resume => {}
                        Admission::Reanchor => self.receive_stats.record_reanchor(),
                        Admission::Reject => {
                            self.metrics_collector.record_ooo_drop();
                            self.receive_stats.record_ooo_drop();
                            continue;
                        }
                    }

                    let current_capacity = self.adaptation_engine.current_capacity();
                    if self.packet_ring.len() >= current_capacity {
                        self.metrics_collector.record_overflow_drop();
                        self.receive_stats.record_overflow_drop();
                        if !self.packet_ring.is_empty() {
                            self.packet_ring.pop_front();
                        }
                    }

                    let recorded_at_ms = self.recording_stamp();
                    self.packet_ring.push_back(RingEntry {
                        packet,
                        recorded_at_ms,
                    });
                    self.warmup
                        .record_packet(self.adaptation_engine.warmup_packets_needed());

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
                .record_adaptation(self.admission.last());

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

    /// Process next packet from ring
    fn process_next_packet(&mut self) -> Option<f32> {
        let target = self.adaptation_engine.warmup_packets_needed();

        while let Some(entry) = self.packet_ring.pop_front() {
            self.starved_frames = 0;
            // The candidate has not played yet, so it counts toward the depth being judged.
            let depth = self.packet_ring.len() + 1;

            match self.audio_processor.decode_held(&entry.packet.data) {
                Ok(candidate_rms) => {
                    self.emit_recording(&entry);

                    // Never the last queued frame: shedding it would starve the buffer rather
                    // than drain it.
                    if depth > 1 && self.drain.should_shed(depth, target, candidate_rms) {
                        self.audio_processor.discard_held();
                        self.receive_stats.record_drain_shed();
                        continue;
                    }

                    let frames_written = self.audio_processor.commit_held();
                    self.audio_processor.reset_plc_counter();
                    self.metrics_collector.record_decode_success(frames_written);
                    self.receive_stats.record_decode(frames_written);

                    // Assessment network conditions after successful decode
                    self.adaptation_engine
                        .assess_network_conditions(&self.metrics_collector);

                    return self.audio_processor.next_sample();
                }
                Err(e) => {
                    error!("Failed to process packet: {}", e);
                    // A packet arrived and could not be used. Concealment either way.
                    return self.generate_plc_sample(true);
                }
            }
        }

        // An empty ring when playback needs a frame is the underrun. Recording it here
        // gives `NetworkMetrics::buffer_underruns` its first writer, so
        // `CongestionLevel::from_buffer_metrics` stops reading a constant zero.
        //
        // It feeds congestion assessment only. Capacity is an overflow backstop far above the
        // playout depth, which the warmup gate and the drain policy set.
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
        if self
            .warmup
            .record_starved_frame(self.starved_frames, Self::CONCEAL_GRACE_FRAMES)
        {
            self.receive_stats.record_warmup_rearm();
        }
        let concealing = self.starved_frames <= Self::CONCEAL_GRACE_FRAMES;
        if concealing {
            self.receive_stats.record_underrun();
        }
        self.generate_plc_sample(concealing)
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

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // Stamped at arrival: this is when the audio was meant to be heard, before the buffer's delay.
    fn recording_stamp(&self) -> Option<u64> {
        let active = self
            .recording_active
            .as_ref()
            .map_or(false, |f| f.load(Ordering::SeqCst));
        (active && self.recording_producer.is_some()).then(Self::now_ms)
    }

    // Emitted whether or not the flag is still on: a frame stamped while recording was active is
    // recorded, so a recording is not abandoned by the flag turning off mid-stream.
    fn emit_recording(&self, entry: &RingEntry) {
        let (Some(recorded_at_ms), Some(producer)) =
            (entry.recorded_at_ms, self.recording_producer.as_ref())
        else {
            return;
        };
        let _ = producer.try_send(RawRecordingData::OutputData {
            absolute_timestamp_ms: Some(recorded_at_ms),
            opus_data: entry.packet.data.to_vec(),
            sample_rate: entry.packet.sample_rate,
            channels: 1,
            emitter: entry.packet.emitter.clone(),
            listener: entry.packet.listener.clone(),
            is_spatial: entry.packet.route == AudioSinkType::Spatial,
        });
    }
}

impl Iterator for JitterBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.audio_processor.next_sample() {
            return Some(sample);
        }

        // Drain incoming packets
        self.drain_incoming();

        if self.stopped && self.packet_ring.is_empty() && !self.audio_processor.has_samples() {
            return None;
        }

        // While the gate is closed, play a frame of silence. Queuing a whole frame, rather than
        // answering each sample with 0.0, means the incoming channel is drained once per 20 ms,
        // as it is when the gate is open, instead of on every sample of the audio thread.
        let warmup_needed = self.adaptation_engine.warmup_packets_needed();
        if !self.warmup.is_open(warmup_needed) {
            self.audio_processor.push_silence_frame();
            return self.audio_processor.next_sample().or(Some(0.0));
        }

        self.process_next_packet()
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
