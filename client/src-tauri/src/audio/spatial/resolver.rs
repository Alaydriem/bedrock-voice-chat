use common::structs::recording::RecordingHeader;
use common::traits::player_data::PlayerData;

use super::{PerceptualGain, SpatialCalculator, SpatialGains};
use crate::audio::recording::renderer::SpatialRenderSettings;

/// One recorded frame's header to the gains it was heard at.
///
/// The only type that knows a frame can fail to be positioned. A `None` is not an error: your own
/// voice, a frame that took the flat route, and a frame recorded before the local player reached
/// the position cache all produce one.
pub struct SpatialResolver {
    settings: SpatialRenderSettings,
}

impl SpatialResolver {
    pub fn new(settings: SpatialRenderSettings) -> Self {
        Self { settings }
    }

    pub fn gains(&self, header: &RecordingHeader) -> Option<SpatialGains> {
        let RecordingHeader::Output(header) = header else {
            return None;
        };

        if !header.is_spatial {
            return None;
        }

        let emitter = header.emitter_metadata.player_data.as_ref()?;
        let listener = header.listener_metadata.player_data.as_ref()?;

        let spatial = SpatialCalculator::gains(
            emitter.get_position(),
            emitter.is_deafened(),
            listener.get_position(),
            listener.get_orientation(),
            listener.get_game(),
            self.settings.config(),
        );

        let gain = match header.emitter_metadata.gain_settings.as_ref() {
            Some(settings) if settings.muted => 0.0,
            Some(settings) => PerceptualGain::amplitude(settings.gain),
            None => 1.0,
        };

        Some(SpatialGains::from_pan(
            spatial.pan,
            spatial.volume * gain,
            self.settings.panning_intensity(),
        ))
    }
}
