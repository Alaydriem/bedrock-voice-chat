/// A linear slider position as an amplitude factor.
///
/// The power curve makes equal slider increments produce roughly equal loudness changes.
pub struct PerceptualGain;

impl PerceptualGain {
    pub fn amplitude(linear_position: f32) -> f32 {
        linear_position.powf(2.5)
    }
}
