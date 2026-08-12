use common::structs::SpatialAudioConfig;
use common::{Coordinate, Game, Orientation};

use super::SpatialAudioData;

/// Where a voice sits relative to the listener, as a pan position and a volume.
///
/// The one definition of that arithmetic. Playback reads it per packet and a render reads it
/// per recorded frame, so a rendered file places a voice exactly where the listener heard it.
pub struct SpatialCalculator;

impl SpatialCalculator {
    pub fn gains(
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
