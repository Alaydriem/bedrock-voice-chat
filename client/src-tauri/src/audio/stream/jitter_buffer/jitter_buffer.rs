use log::error;
use rodio::Source;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::jitter_buffer_source::{JitterBufferError, JitterBufferSource};
use super::EncodedAudioFramePacket;
use common::{Coordinate, Orientation};

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

pub struct JitterBuffer {
    source: Arc<Mutex<JitterBufferSource>>,
}

impl JitterBuffer {
    /// Create a new JitterBuffer pair with activity detection support
    pub fn create_with_handle_and_activity(
        initial_packet: EncodedAudioFramePacket,
        identifier: String,
        player_name: String,
        activity_tx: Option<flume::Sender<crate::audio::stream::ActivityUpdate>>,
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

        let source = super::jitter_buffer_source::JitterBufferSource::new_with_activity(
            rx,
            initial_packet,
            buffer_capacity,
            player_name,
            activity_tx,
        )?;

        let jitter_buffer = Self {
            source: Arc::new(Mutex::new(source)),
        };

        let handle = JitterBufferHandle { tx };
        Ok((jitter_buffer, handle))
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

impl Source for JitterBuffer {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        if let Ok(source) = self.source.lock() {
            source.channels()
        } else {
            1
        }
    }

    fn sample_rate(&self) -> u32 {
        if let Ok(source) = self.source.lock() {
            source.sample_rate()
        } else {
            48000
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
