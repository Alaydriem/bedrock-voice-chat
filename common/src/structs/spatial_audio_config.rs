use serde::{Deserialize, Serialize};
use ts_rs::TS;

fn default_broadcast_range() -> f32 {
    48.0
}

fn default_close_threshold() -> f32 {
    12.0
}

fn default_falloff_distance() -> f32 {
    48.0
}

fn default_steepen_start() -> f32 {
    38.0
}

fn default_deafen_distance() -> f32 {
    3.0
}

fn default_panning_start() -> f32 {
    8.0
}

fn default_max_attenuation_db() -> f32 {
    40.0
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SpatialAudioConfig {
    #[serde(default = "default_broadcast_range")]
    pub broadcast_range: f32,
    #[serde(default = "default_close_threshold")]
    pub close_threshold: f32,
    #[serde(default = "default_falloff_distance")]
    pub falloff_distance: f32,
    #[serde(default = "default_steepen_start")]
    pub steepen_start: f32,
    #[serde(default = "default_deafen_distance")]
    pub deafen_distance: f32,
    #[serde(default = "default_panning_start")]
    pub panning_start: f32,
    #[serde(default = "default_max_attenuation_db")]
    pub max_attenuation_db: f32,
}

impl Default for SpatialAudioConfig {
    fn default() -> Self {
        Self {
            broadcast_range: default_broadcast_range(),
            close_threshold: default_close_threshold(),
            falloff_distance: default_falloff_distance(),
            steepen_start: default_steepen_start(),
            deafen_distance: default_deafen_distance(),
            panning_start: default_panning_start(),
            max_attenuation_db: default_max_attenuation_db(),
        }
    }
}
