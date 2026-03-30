use log::error;
use rodio::Source;
use std::num::NonZero;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::EncodedAudioFramePacket;
use super::jitter_buffer_source::{JitterBufferError, JitterBufferSource};
use crate::audio::recording::RecordingProducer;
use common::structs::SpatialAudioConfig;
use common::{Coordinate, Game, Orientation};

#[derive(Debug, Clone)]
pub struct SpatialAudioData {
    // +1.0 = left, -1.0 = right
    pub pan: f32,
    // 0.0 to 1.0, distance-based
    pub volume: f32,
}

#[derive(Clone)]
pub struct JitterBufferHandle {
    tx: flume::Sender<Option<EncodedAudioFramePacket>>,
}

impl JitterBufferHandle {
    pub fn enqueue(&self, packet: EncodedAudioFramePacket) -> Result<(), JitterBufferError> {
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
    /// Create a new JitterBuffer pair with activity detection and recording support
    pub fn create_with_handle_and_activity(
        initial_packet: EncodedAudioFramePacket,
        identifier: String,
        player_name: String,
        activity_tx: Option<flume::Sender<crate::audio::stream::ActivityUpdate>>,
        recording_producer: Option<RecordingProducer>,
        recording_active: Option<Arc<AtomicBool>>,
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
            recording_producer,
            recording_active,
        )?;

        let jitter_buffer = Self {
            source: Arc::new(Mutex::new(source)),
        };

        let handle = JitterBufferHandle { tx };
        Ok((jitter_buffer, handle))
    }

    pub fn calculate_spatial_audio_data(
        emitter: &Coordinate,
        deafen_emitter: bool,
        listener: &Coordinate,
        orientation: &Orientation,
        game: Game,
        config: &SpatialAudioConfig,
    ) -> SpatialAudioData {
        let dx = emitter.x - listener.x;
        let dy = emitter.y - listener.y;
        let dz = emitter.z - listener.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Deafen: server enforces deafen_distance, so if we receive the packet just play it
        if deafen_emitter {
            return SpatialAudioData {
                pan: 0.0,
                volume: 1.0,
            };
        }

        // Beyond falloff: silence
        if distance > config.falloff_distance {
            return SpatialAudioData {
                pan: 0.0,
                volume: 0.0,
            };
        }

        // Pan: dot product of XZ direction with listener's left vector
        let raw_pan = if distance > 0.01 {
            let dir_x = dx / distance;
            let dir_z = dz / distance;

            let yaw_rad = orientation.y.to_radians();
            let (left_x, left_z) = match game {
                // Minecraft: yaw 0 = South (+Z), clockwise
                Game::Minecraft => (yaw_rad.cos(), yaw_rad.sin()),
                // Hytale: yaw 0 = North (-Z), counter-clockwise
                Game::Hytale => (-yaw_rad.cos(), yaw_rad.sin()),
            };

            dir_x * left_x + dir_z * left_z
        } else {
            0.0
        };

        // Suppress panning at close range
        let proximity_factor = if distance <= config.panning_start {
            0.0
        } else if distance <= config.close_threshold {
            (distance - config.panning_start) / (config.close_threshold - config.panning_start)
        } else {
            1.0
        };
        let pan = raw_pan * proximity_factor.clamp(0.0, 1.0);

        // dB-based volume attenuation
        let volume = if distance <= config.close_threshold {
            1.0
        } else {
            let t = (distance - config.close_threshold)
                / (config.falloff_distance - config.close_threshold);
            let db_atten = t * config.max_attenuation_db;
            let mut vol = 10.0_f32.powf(-db_atten / 20.0);

            if distance >= config.steepen_start {
                let s = (distance - config.steepen_start)
                    / (config.falloff_distance - config.steepen_start);
                vol *= 1.0 - s;
            }

            vol
        };

        SpatialAudioData { pan, volume }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SpatialAudioConfig {
        SpatialAudioConfig::default()
    }

    fn listener_at_origin() -> Coordinate {
        Coordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn minecraft_facing_south_emitter_east_pans_left() {
        // Minecraft: yaw 0 = facing south (+Z). Emitter to the east (+X) should be on the left.
        let emitter = Coordinate {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan > 0.5,
            "Expected positive pan (left), got {}",
            result.pan
        );
    }

    #[test]
    fn minecraft_facing_south_emitter_west_pans_right() {
        let emitter = Coordinate {
            x: -20.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan < -0.5,
            "Expected negative pan (right), got {}",
            result.pan
        );
    }

    #[test]
    fn minecraft_facing_south_emitter_ahead_centered() {
        let emitter = Coordinate {
            x: 0.0,
            y: 0.0,
            z: 20.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan.abs() < 0.01,
            "Expected centered pan, got {}",
            result.pan
        );
    }

    #[test]
    fn minecraft_facing_south_emitter_behind_centered() {
        let emitter = Coordinate {
            x: 0.0,
            y: 0.0,
            z: -20.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan.abs() < 0.01,
            "Expected centered pan, got {}",
            result.pan
        );
    }

    #[test]
    fn minecraft_facing_west_emitter_south_pans_left() {
        // yaw 90 = facing west (-X). Emitter to the south (+Z) is on the left.
        let emitter = Coordinate {
            x: 0.0,
            y: 0.0,
            z: 20.0,
        };
        let orientation = Orientation { x: 0.0, y: 90.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan > 0.5,
            "Expected positive pan (left), got {}",
            result.pan
        );
    }

    #[test]
    fn hytale_facing_north_emitter_west_pans_left() {
        // Hytale: yaw 0 = facing north (-Z). Emitter to the west (-X) should be on the left.
        let emitter = Coordinate {
            x: -20.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Hytale,
            &default_config(),
        );
        assert!(
            result.pan > 0.5,
            "Expected positive pan (left), got {}",
            result.pan
        );
    }

    #[test]
    fn hytale_facing_north_emitter_east_pans_right() {
        let emitter = Coordinate {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Hytale,
            &default_config(),
        );
        assert!(
            result.pan < -0.5,
            "Expected negative pan (right), got {}",
            result.pan
        );
    }

    #[test]
    fn close_range_suppresses_panning() {
        // At 5 units, within panning_start (8.0) -> no panning
        let emitter = Coordinate {
            x: 5.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan.abs() < 0.01,
            "Expected suppressed pan at close range, got {}",
            result.pan
        );
        assert!(
            (result.volume - 1.0).abs() < 0.01,
            "Expected full volume at close range, got {}",
            result.volume
        );
    }

    #[test]
    fn mid_range_ramps_panning() {
        // At 10 units, between panning_start (8.0) and close_threshold (12.0) -> partial panning
        let emitter = Coordinate {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.pan > 0.0 && result.pan < 1.0,
            "Expected partial pan, got {}",
            result.pan
        );
    }

    #[test]
    fn beyond_falloff_is_silent() {
        let emitter = Coordinate {
            x: 50.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &default_config(),
        );
        assert!(
            result.volume < 0.001,
            "Expected silence beyond falloff, got {}",
            result.volume
        );
    }

    #[test]
    fn volume_attenuates_with_distance() {
        let config = default_config();
        let orientation = Orientation { x: 0.0, y: 0.0 };

        let near = JitterBuffer::calculate_spatial_audio_data(
            &Coordinate {
                x: 0.0,
                y: 0.0,
                z: 15.0,
            },
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &config,
        );
        let far = JitterBuffer::calculate_spatial_audio_data(
            &Coordinate {
                x: 0.0,
                y: 0.0,
                z: 35.0,
            },
            false,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &config,
        );

        assert!(
            near.volume > far.volume,
            "Near volume {} should exceed far volume {}",
            near.volume,
            far.volume
        );
        assert!(near.volume > 0.0 && near.volume < 1.0);
        assert!(far.volume > 0.0);
    }

    #[test]
    fn deafen_plays_at_full_volume() {
        let config = default_config();
        let emitter = Coordinate {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        };
        let orientation = Orientation { x: 0.0, y: 0.0 };
        let result = JitterBuffer::calculate_spatial_audio_data(
            &emitter,
            true,
            &listener_at_origin(),
            &orientation,
            Game::Minecraft,
            &config,
        );
        assert!((result.volume - 1.0).abs() < 0.01);
        assert!(result.pan.abs() < 0.01);
    }
}
