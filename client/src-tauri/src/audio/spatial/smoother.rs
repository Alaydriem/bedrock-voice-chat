use super::SpatialGains;

/// A one-pole ramp toward a target gain pair, advanced once per mono sample.
///
/// Position updates arrive every 20 ms. Applying them as steps clicks; this is what turns each
/// step into a ramp short enough not to smear and long enough not to be heard.
pub struct GainSmoother {
    left: f32,
    right: f32,
    volume: f32,
}

impl GainSmoother {
    // ~4.2ms time constant at 48kHz
    pub const SMOOTH_COEFF: f32 = 0.005;

    // After this many samples the remaining error is below 1e-9, so a longer gap can be snapped
    // rather than walked.
    pub const SETTLE_SAMPLES: usize = 4096;

    pub fn new(initial: SpatialGains) -> Self {
        Self {
            left: initial.left,
            right: initial.right,
            volume: initial.volume,
        }
    }

    pub fn advance(&mut self, target: &SpatialGains) -> SpatialGains {
        self.left += (target.left - self.left) * Self::SMOOTH_COEFF;
        self.right += (target.right - self.right) * Self::SMOOTH_COEFF;
        self.volume += (target.volume - self.volume) * Self::SMOOTH_COEFF;

        SpatialGains {
            left: self.left,
            right: self.right,
            volume: self.volume,
        }
    }

    /// Advance without producing anything, for a stretch of the timeline that carries no samples
    /// of its own.
    pub fn advance_by(&mut self, target: &SpatialGains, samples: usize) {
        if samples >= Self::SETTLE_SAMPLES {
            self.snap(target);
            return;
        }

        for _ in 0..samples {
            self.advance(target);
        }
    }

    fn snap(&mut self, target: &SpatialGains) {
        self.left = target.left;
        self.right = target.right;
        self.volume = target.volume;
    }
}
