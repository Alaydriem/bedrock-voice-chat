/// Generates a continuous 440 Hz sine as mono 48 kHz f32 frames of 960 samples
/// (20 ms each), keeping phase across frames so the tone is seamless. Fed to a
/// bot as `InputPcm`; the client bin paces the 20 ms cadence itself.
pub struct Tone {
    phase: f64,
    step: f64,
}

impl Tone {
    const SAMPLE_RATE: f64 = 48_000.0;
    const FREQ: f64 = 440.0;
    const AMPLITUDE: f32 = 0.3;
    pub const FRAME_SAMPLES: usize = 960;

    pub fn new() -> Self {
        Self {
            phase: 0.0,
            step: std::f64::consts::TAU * Self::FREQ / Self::SAMPLE_RATE,
        }
    }

    /// Next 20 ms frame of samples.
    pub fn next_frame(&mut self) -> Vec<f32> {
        let mut out = Vec::with_capacity(Self::FRAME_SAMPLES);
        for _ in 0..Self::FRAME_SAMPLES {
            out.push((self.phase.sin() as f32) * Self::AMPLITUDE);
            self.phase += self.step;
            if self.phase >= std::f64::consts::TAU {
                self.phase -= std::f64::consts::TAU;
            }
        }
        out
    }
}
