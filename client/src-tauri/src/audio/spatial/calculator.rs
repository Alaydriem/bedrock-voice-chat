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
        whispering: bool,
        listener: &Coordinate,
        orientation: &Orientation,
        game: Game,
        config: &SpatialAudioConfig,
    ) -> SpatialAudioData {
        let dx = emitter.x - listener.x;
        let dy = emitter.y - listener.y;
        let dz = emitter.z - listener.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        let Some(curve_distance) = Self::curve_distance(distance, whispering, config) else {
            return SpatialAudioData {
                pan: 0.0,
                volume: 0.0,
            };
        };

        // Beyond falloff: silence
        if curve_distance > config.falloff_distance {
            return SpatialAudioData {
                pan: 0.0,
                volume: 0.0,
            };
        }

        // Direction from the real offset; how far along the curve from `curve_distance`.
        let raw_pan = if distance > 0.01 {
            let dir_x = dx / distance;
            let dir_z = dz / distance;

            let yaw_rad = orientation.y.to_radians();
            let (left_x, left_z) = match game {
                // Minecraft: yaw 0 = South (+Z), clockwise
                Game::Minecraft => (yaw_rad.cos(), yaw_rad.sin()),
            };

            dir_x * left_x + dir_z * left_z
        } else {
            0.0
        };

        // Suppress panning at close range
        let proximity_factor = if curve_distance <= config.panning_start {
            0.0
        } else if curve_distance <= config.close_threshold {
            (curve_distance - config.panning_start)
                / (config.close_threshold - config.panning_start)
        } else {
            1.0
        };
        let pan = raw_pan * proximity_factor.clamp(0.0, 1.0);

        // dB-based volume attenuation
        let volume = if curve_distance <= config.close_threshold {
            1.0
        } else {
            let t = (curve_distance - config.close_threshold)
                / (config.falloff_distance - config.close_threshold);
            let db_atten = t * config.max_attenuation_db;
            let mut vol = 10.0_f32.powf(-db_atten / 20.0);

            if curve_distance >= config.steepen_start {
                let s = (curve_distance - config.steepen_start)
                    / (config.falloff_distance - config.steepen_start);
                vol *= 1.0 - s;
            }

            vol
        };

        SpatialAudioData { pan, volume }
    }

    /// Where on the normal curve a listener at `distance` sits, or `None` for silence.
    ///
    /// A whisper's whole range maps onto the whole curve, so it fades to silence at the edge
    /// the server stops sending at instead of cutting off there. An edge of zero or less, or
    /// one that is not a number, is a whisper nobody hears.
    fn curve_distance(distance: f32, whispering: bool, config: &SpatialAudioConfig) -> Option<f32> {
        if !whispering {
            return Some(distance);
        }

        let edge = config.whisper_edge();
        if !(edge > 0.0) || distance > edge {
            return None;
        }

        Some(distance * config.falloff_distance / edge)
    }
}
