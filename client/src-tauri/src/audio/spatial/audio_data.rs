#[derive(Debug, Clone, Copy)]
pub struct SpatialAudioData {
    // +1.0 = left, -1.0 = right
    pub pan: f32,
    // 0.0 to 1.0, distance-based
    pub volume: f32,
}
