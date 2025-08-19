use log::{error, warn};
use opus::{Channels, Decoder};
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer}};
use rodio::Source;
use std::collections::VecDeque;
use std::time::Duration;

use super::EncodedAudioFramePacket;
use common::{Coordinate, Orientation};

const MAX_DECODE_ERRORS: usize = 10;
const WARMUP_PACKETS_NEEDED: usize = 2;
const MAX_PLC_ATTEMPTS: usize = 5;
const MAX_OPUS_FRAME_MS: usize = 480;
const FRAME_MS: u64 = 20;
const LARGE_JUMP_FORWARD_MS: u64 = 1000;

#[derive(Debug)]
pub enum JitterBufferError {
    DecoderError(opus::Error),
    InvalidPacket,
}

impl std::fmt::Display for JitterBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitterBufferError::DecoderError(e) => write!(f, "Opus decoder error: {:?}", e),
            JitterBufferError::InvalidPacket => write!(f, "Invalid packet data"),
        }
    }
}

impl std::error::Error for JitterBufferError {}

#[derive(Debug, Clone)]
pub struct SpatialAudioData {
    #[allow(dead_code)]
    pub emitter: Coordinate,
    pub left_ear: Coordinate,
    pub right_ear: Coordinate,
    pub gain: f32,
}

#[derive(Clone)]
pub struct JitterBufferHandle {
    tx: flume::Sender<Option<EncodedAudioFramePacket>>,
}

impl JitterBufferHandle {
    pub fn enqueue(
        &self,
        packet: EncodedAudioFramePacket,
        _spatial: Option<SpatialAudioData>,
    ) -> Result<(), JitterBufferError> {
        self.tx
            .send(Some(packet))
            .map_err(|_| JitterBufferError::InvalidPacket)
    }

    pub fn stop(&self) {
        // Send None to indicate stop
        let _ = self.tx.send(None);
    }
}

pub struct JitterBufferSource {
    rx: flume::Receiver<Option<EncodedAudioFramePacket>>,
    ring: VecDeque<EncodedAudioFramePacket>,
    decoder: Decoder,
    current_sample_rate: u32,
    ring_high_water: usize,
    last_ring_len: usize,
    warmup_packets_received: usize,
    plc_consecutive_count: usize,
    decode_error_count: usize,
    decode_buffer: Vec<f32>,
    output_producer: ringbuf::HeapProd<f32>,
    output_consumer: ringbuf::HeapCons<f32>,
    capacity: usize,
    stopped: bool,
    last_output_ts_ms: u64,
    last_accepted_timestamp: u64,
    samples_per_frame: usize,
    frame_sample_countdown: usize,
    queued_frames: usize,
    frames_decoded: u64,
    frames_plc: u64,
    frames_silence: u64,
    frames_dropped_overflow: u64,
    frames_dropped_ooo: u64,
    aggregated_decodes: u64,
}

impl JitterBufferSource {
    fn frames_for_rate(rate: u32) -> usize { (rate as usize) / 50 }
    fn max_samples_for_rate(rate: u32) -> usize { (rate as usize) * MAX_OPUS_FRAME_MS / 1000 }

    fn new(
        rx: flume::Receiver<Option<EncodedAudioFramePacket>>,
        capacity: usize,
        initial_packet: EncodedAudioFramePacket,
    ) -> Result<Self, JitterBufferError> {
        let initial_rate = initial_packet.sample_rate as u32;
        let decoder =
            Decoder::new(initial_rate, Channels::Mono).map_err(JitterBufferError::DecoderError)?;

    let frames = Self::frames_for_rate(initial_rate);

    // Allocate for up to 120 ms to avoid truncation when packets aggregate frames
    let max_samples = Self::max_samples_for_rate(initial_rate);
    let mut decode_buffer = Vec::with_capacity(max_samples);
    decode_buffer.resize(max_samples, 0.0);

    let ring_buf = HeapRb::<f32>::new(frames * 4);
    let (output_producer, output_consumer) = ring_buf.split();

        let mut source = Self {
            rx,
            ring: VecDeque::with_capacity(capacity),
            decoder,
            current_sample_rate: initial_rate,
            ring_high_water: 0,
            last_ring_len: 0,
            warmup_packets_received: 0,
            plc_consecutive_count: 0,
            decode_error_count: 0,
            decode_buffer,
            output_producer,
            output_consumer,
            capacity,
            stopped: false,
            last_output_ts_ms: initial_packet.timestamp.saturating_sub(FRAME_MS),
            last_accepted_timestamp: initial_packet.timestamp,
            samples_per_frame: frames,
            frame_sample_countdown: 0,
            queued_frames: 0,
            frames_decoded: 0,
            frames_plc: 0,
            frames_silence: 0,
            aggregated_decodes: 0,
            frames_dropped_overflow: 0,
            frames_dropped_ooo: 0,
        };

        if source.ring.len() >= source.capacity {
            source.ring.pop_front();
        }
        source.ring.push_back(initial_packet);
        source.warmup_packets_received =
            (source.warmup_packets_received + 1).min(WARMUP_PACKETS_NEEDED);

        Ok(source)
    }

    fn reset_decoder(&mut self) -> Result<(), JitterBufferError> {
        self.decoder = Decoder::new(self.current_sample_rate, Channels::Mono)
            .map_err(JitterBufferError::DecoderError)?;
        self.decode_error_count = 0;
        warn!("Decoder reset due to consecutive errors");
        Ok(())
    }

    fn process_packet_sync(
        &mut self,
        packet: &EncodedAudioFramePacket,
    ) -> Result<usize, JitterBufferError> {
        self.decode_opus(&packet.data)
    }

    fn decode_opus(&mut self, opus_data: &[u8]) -> Result<usize, JitterBufferError> {
        match self
            .decoder
            .decode_float(opus_data, &mut self.decode_buffer, false)
        {
            Ok(samples_written) => {
                self.decode_error_count = 0;
                let expected_20ms = Self::frames_for_rate(self.current_sample_rate);
                if samples_written > expected_20ms * 2 {
                    warn!(
                        "Opus packet decoded {} samples (> 40ms) at {} Hz; sender may be aggregating frames",
                        samples_written,
                        self.current_sample_rate
                    );
                    self.aggregated_decodes += 1;
                }
                Ok(samples_written)
            }
            Err(e) => {
                self.decode_error_count += 1;
                error!("Opus decode error: {:?}", e);

                if self.decode_error_count >= MAX_DECODE_ERRORS {
                    self.reset_decoder()?;
                }

                Err(JitterBufferError::DecoderError(e))
            }
        }
    }

    fn generate_plc_to_ring(&mut self) -> usize {
        if self.plc_consecutive_count >= MAX_PLC_ATTEMPTS {
            let mut written = 0;
            for _ in 0..self.samples_per_frame {
                if self.output_producer.try_push(0.0).is_err() {
                    break;
                }
                written += 1;
            }
            written
        } else {
            match self
                .decoder
                .decode_float(&[], &mut self.decode_buffer, false)
            {
                Ok(samples_written) => {
                    self.plc_consecutive_count += 1;
                    let copy_len = samples_written.min(self.samples_per_frame);
                    
                    let mut written = 0;
                    for &sample in &self.decode_buffer[..copy_len] {
                        if self.output_producer.try_push(sample).is_err() {
                            break;
                        }
                        written += 1;
                    }
                    
                    for _ in copy_len..self.samples_per_frame {
                        if self.output_producer.try_push(0.0).is_err() {
                            break;
                        }
                        written += 1;
                    }
                    written
                }
                Err(e) => {
                    error!("PLC generation error: {:?}", e);
                    let mut written = 0;
                    for _ in 0..self.samples_per_frame {
                        if self.output_producer.try_push(0.0).is_err() {
                            break;
                        }
                        written += 1;
                    }
                    written
                }
            }
        }
    }

    fn is_packet_acceptable(&self, packet_timestamp: u64) -> bool {
        if packet_timestamp <= self.last_accepted_timestamp {
            return false;
        }

        let time_diff = packet_timestamp.saturating_sub(self.last_accepted_timestamp);
        if time_diff > LARGE_JUMP_FORWARD_MS {
            return true;
        }

        true
    }

    fn drain_incoming(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Some(packet) => {
                    let packet_timestamp = packet.timestamp;
                    
                    if !self.is_packet_acceptable(packet_timestamp) {
                        self.frames_dropped_ooo = self.frames_dropped_ooo.saturating_add(1);
                        if self.frames_dropped_ooo % 50 == 1 {
                            warn!(
                                "Out-of-order: dropping packet ts={} <= last_accepted={} (drops={})",
                                packet_timestamp, self.last_accepted_timestamp, self.frames_dropped_ooo
                            );
                        }
                        continue;
                    }

                    self.last_accepted_timestamp = packet_timestamp;

                    if self.ring.len() >= self.capacity {
                        self.frames_dropped_overflow = self.frames_dropped_overflow.saturating_add(1);
                        if self.frames_dropped_overflow % 100 == 1 {
                            warn!(
                                "Overflow: dropping incoming packet to preserve smooth playback (drops={}, cap={})",
                                self.frames_dropped_overflow, self.capacity
                            );
                        }
                    } else {
                        self.ring.push_back(packet);
                        let len = self.ring.len();
                        if len > self.ring_high_water { self.ring_high_water = len; }
                        self.last_ring_len = len;
                        self.warmup_packets_received = (self.warmup_packets_received + 1).min(WARMUP_PACKETS_NEEDED);
                    }
                }
                None => {
                    self.stopped = true;
                }
            }
        }
    }
}

impl Iterator for JitterBufferSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.output_consumer.try_pop() {
            if self.frame_sample_countdown == 0 {
                self.frame_sample_countdown = self.samples_per_frame;
            }
            self.frame_sample_countdown = self.frame_sample_countdown.saturating_sub(1);
            if self.frame_sample_countdown == 0 {
                self.last_output_ts_ms = self.last_output_ts_ms.saturating_add(FRAME_MS);
                if self.queued_frames > 0 { self.queued_frames -= 1; }
            }
            return Some(sample);
        }

        self.drain_incoming();

        if self.stopped && self.ring.is_empty() {
            return None;
        }

        if self.warmup_packets_received < WARMUP_PACKETS_NEEDED {
            return Some(0.0);
        }

        if let Some(packet) = self.ring.pop_front() {
            match self.process_packet_sync(&packet) {
                Ok(samples_written) => {
                    self.plc_consecutive_count = 0;
                    
                    let spf = self.samples_per_frame;
                    let mut frames_added = 0usize;
                    for chunk in self.decode_buffer[..samples_written].chunks(spf) {
                        for &sample in chunk {
                            if self.output_producer.try_push(sample).is_err() {
                                warn!("Ring buffer overflow during decode");
                                break;
                            }
                        }
                        
                        if chunk.len() < spf {
                            for _ in chunk.len()..spf {
                                if self.output_producer.try_push(0.0).is_err() {
                                    break;
                                }
                            }
                        }
                        frames_added += 1;
                    }
                    
                    if frames_added > 0 {
                        self.queued_frames = self.queued_frames.saturating_add(frames_added);
                        self.frames_decoded += frames_added as u64;
                    }
                    self.output_consumer.try_pop()
                }
                Err(e) => {
                    error!("Failed to process packet: {}", e);
                    if self.plc_consecutive_count >= MAX_PLC_ATTEMPTS {
                        self.frames_silence += 1;
                    } else {
                        self.frames_plc += 1;
                    }
                    self.generate_plc_to_ring();
                    self.queued_frames = self.queued_frames.saturating_add(1);
                    self.output_consumer.try_pop()
                }
            }
        } else {
            if self.plc_consecutive_count >= MAX_PLC_ATTEMPTS {
                self.frames_silence += 1;
            } else {
                self.frames_plc += 1;
            }
            self.generate_plc_to_ring();
            self.queued_frames = self.queued_frames.saturating_add(1);
            self.output_consumer.try_pop()
        }
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
        self.current_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite stream
    }
}
pub struct JitterBuffer;

impl JitterBuffer {
    /// Create a new JitterBuffer pair (source, handle) seeded with the first packet
    pub fn new(
        initial_packet: EncodedAudioFramePacket,
        capacity: usize,
    ) -> Result<(JitterBufferSource, JitterBufferHandle), JitterBufferError> {
        let (tx, rx) = flume::unbounded::<Option<EncodedAudioFramePacket>>();
        let source = JitterBufferSource::new(rx, capacity, initial_packet)?;
        let handle = JitterBufferHandle { tx };
        Ok((source, handle))
    }

    pub fn calculate_virtual_listener_audio_data(
        emitter: &Coordinate,
        deafen_emitter: bool,
        listener: &Coordinate,
        orientation: &Orientation,
    ) -> SpatialAudioData {
        // Compute delta and full 3D distance for gain
        let dx = emitter.x - listener.x;
        let dy = emitter.y - listener.y;
        let dz = emitter.z - listener.z;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Constants
        let virtual_distance = 1.33;
        let close_threshold = 12.0;
        let falloff_distance = 48.0;
        let steepen_start = 38.0;
        let deafen_distance = 3.0;
        let deafen_multiplier = 0.35;

        let target_min_volume = 1.0 / (12.0 * 12.0);
        let target_max_volume = 1.0 / (virtual_distance * virtual_distance);

        // Direction vector in full 3D
        let direction = if distance > 0.01 {
            [dx / distance, dy / distance, dz / distance]
        } else {
            [0.0, 0.0, -1.0]
        };

        // Virtual listener position logic
        let virtual_listener = if distance <= close_threshold {
            Coordinate {
                x: emitter.x - direction[0] * virtual_distance,
                y: emitter.y - direction[1] * virtual_distance,
                z: emitter.z - direction[2] * virtual_distance,
            }
        } else if distance <= falloff_distance {
            let t = (distance - close_threshold) / (falloff_distance - close_threshold); // 0 → 1
            let mut volume = target_max_volume + t * (target_min_volume - target_max_volume);

            if distance >= steepen_start {
                let s = (distance - steepen_start) / (falloff_distance - steepen_start); // 0 → 1
                let steep_factor = s.powf(2.0); // steeper near end
                volume *= 1.0 - 0.5 * steep_factor; // reduce volume more aggressively
            }

            let mapped_distance = 1.0 / volume.sqrt();
            Coordinate {
                x: emitter.x - direction[0] * mapped_distance,
                y: emitter.y - direction[1] * mapped_distance,
                z: emitter.z - direction[2] * mapped_distance,
            }
        } else {
            listener.clone()
        };

        // Compute yaw (rotation about Y axis)
        let yaw_rad = orientation.y.to_radians();
        let forward_x = yaw_rad.sin();
        let forward_z = -yaw_rad.cos();
        let left_x = -forward_z;
        let left_z = forward_x;
        let ear_offset = 0.3;

        let mut left_ear = Coordinate {
            x: virtual_listener.x + left_x * ear_offset,
            y: virtual_listener.y,
            z: virtual_listener.z + left_z * ear_offset,
        };
        let mut right_ear = Coordinate {
            x: virtual_listener.x - left_x * ear_offset,
            y: virtual_listener.y,
            z: virtual_listener.z - left_z * ear_offset,
        };

        // There's stereo inversion at 24 units away???
        if distance >= 24.0 {
            right_ear = Coordinate {
                x: virtual_listener.x + left_x * ear_offset,
                y: virtual_listener.y,
                z: virtual_listener.z + left_z * ear_offset,
            };
            left_ear = Coordinate {
                x: virtual_listener.x - left_x * ear_offset,
                y: virtual_listener.y,
                z: virtual_listener.z - left_z * ear_offset,
            };
        }

        // Gain logic
        let gain = match deafen_emitter {
            true => {
                if distance <= deafen_distance {
                    1.0 * deafen_multiplier
                } else {
                    0.0
                }
            }
            false => {
                if distance <= falloff_distance {
                    1.0
                } else {
                    0.0
                }
            }
        };

        SpatialAudioData {
            emitter: emitter.clone(),
            left_ear,
            right_ear,
            gain,
        }
    }
}
