/// A per-ear amplitude pair and the volume it is scaled by.
///
/// The pair is equal-power, so a voice crossing the stereo field holds its loudness.
#[derive(Debug, Clone, Copy)]
pub struct SpatialGains {
    pub left: f32,
    pub right: f32,
    pub volume: f32,
}

impl SpatialGains {
    /// Both ears equal at unity. What a listener whose position is not yet known hears, and the
    /// seed the playback sink starts every spatial voice from.
    pub fn centred() -> Self {
        let equal = 0.5_f32.sqrt();
        Self {
            left: equal,
            right: equal,
            volume: 1.0,
        }
    }

    /// `panning_intensity` narrows the field toward centre; it is the listener's own setting and
    /// does not belong to the geometry.
    pub fn from_pan(pan: f32, volume: f32, panning_intensity: f32) -> Self {
        let scaled = (pan * panning_intensity).clamp(-1.0, 1.0);
        Self {
            left: ((1.0 + scaled) / 2.0).sqrt(),
            right: ((1.0 - scaled) / 2.0).sqrt(),
            volume,
        }
    }
}
