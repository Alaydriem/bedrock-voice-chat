use common::structs::SpatialAudioConfig;

use super::SettingsProvenance;

/// The two session settings a spatial render needs and the recording does not carry.
#[derive(Debug, Clone)]
pub struct SpatialRenderSettings {
    config: SpatialAudioConfig,
    panning_intensity: f32,
    provenance: SettingsProvenance,
}

impl SpatialRenderSettings {
    pub fn new(
        config: SpatialAudioConfig,
        panning_intensity: f32,
        provenance: SettingsProvenance,
    ) -> Self {
        Self {
            config,
            panning_intensity,
            provenance,
        }
    }

    pub fn config(&self) -> &SpatialAudioConfig {
        &self.config
    }

    pub fn panning_intensity(&self) -> f32 {
        self.panning_intensity
    }

    pub fn provenance(&self) -> SettingsProvenance {
        self.provenance
    }
}
