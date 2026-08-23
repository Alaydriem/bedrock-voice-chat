use common::structs::SpatialAudioConfig;
use tauri_plugin_store::StoreExt;

use crate::audio::recording::renderer::{SettingsProvenance, SpatialRenderSettings};
use crate::audio::stream::AudioStreamManager;
use crate::audio::AudioDeviceType;

/// Where a render's spatial settings come from.
///
/// Neither value is in the recording. The live session has them only while a session is up, so a
/// copy is kept in the store for the export that happens after the session ends.
pub struct SpatialSettingsResolver;

impl SpatialSettingsResolver {
    // Matches what the output stream falls back to when the key is unset.
    const DEFAULT_PANNING_INTENSITY: f32 = 0.8;
    const CONFIG_KEY: &'static str = "spatial_audio_config";
    const INTENSITY_KEY: &'static str = "panning_intensity";

    pub fn choose(
        live: Option<(SpatialAudioConfig, f32)>,
        last_known: Option<(SpatialAudioConfig, f32)>,
    ) -> SpatialRenderSettings {
        match (live, last_known) {
            (Some((config, intensity)), _) => {
                SpatialRenderSettings::new(config, intensity, SettingsProvenance::LiveSession)
            }
            (None, Some((config, intensity))) => {
                SpatialRenderSettings::new(config, intensity, SettingsProvenance::LastKnown)
            }
            (None, None) => SpatialRenderSettings::new(
                SpatialAudioConfig::default(),
                Self::DEFAULT_PANNING_INTENSITY,
                SettingsProvenance::Defaults,
            ),
        }
    }

    // Crate-scoped rather than public: `AudioStreamManager` is itself `pub(crate)`, and the seam
    // worth exposing is `choose`, which is where the decision lives.
    pub(crate) async fn live(asm: &AudioStreamManager) -> Option<(SpatialAudioConfig, f32)> {
        let config = asm
            .metadata_value(Self::CONFIG_KEY, &AudioDeviceType::OutputDevice)
            .await?;
        let config = serde_json::from_str::<SpatialAudioConfig>(&config).ok()?;

        let intensity = asm
            .metadata_value(Self::INTENSITY_KEY, &AudioDeviceType::OutputDevice)
            .await
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(Self::DEFAULT_PANNING_INTENSITY);

        Some((config, intensity))
    }

    pub fn last_known(app_handle: &tauri::AppHandle) -> Option<(SpatialAudioConfig, f32)> {
        let store = app_handle.store("store.json").ok()?;

        let config = store.get(Self::CONFIG_KEY)?;
        let config = serde_json::from_value::<SpatialAudioConfig>(config).ok()?;

        let intensity = store
            .get(Self::INTENSITY_KEY)
            .and_then(|value| value.as_f64())
            .map(|value| value as f32)
            .unwrap_or(Self::DEFAULT_PANNING_INTENSITY);

        Some((config, intensity))
    }
}
