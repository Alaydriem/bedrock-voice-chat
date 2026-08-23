#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkQuality {
    // < 1% loss, < 20ms jitter, stable RTT
    Excellent,
    // < 3% loss, < 50ms jitter, moderate RTT variance
    Good,
    // < 8% loss, < 100ms jitter, high RTT variance
    Moderate,
    // > 8% loss, > 100ms jitter, very unstable
    Poor,
}

impl NetworkQuality {
    /// Assess network quality from metrics
    pub fn from_metrics(loss_rate: f64, jitter_ms: f64, _rtt_variance: f64) -> Self {
        match (loss_rate, jitter_ms) {
            (loss, jitter) if loss < 0.01 && jitter < 20.0 => NetworkQuality::Excellent,
            (loss, jitter) if loss < 0.03 && jitter < 50.0 => NetworkQuality::Good,
            (loss, jitter) if loss < 0.08 && jitter < 100.0 => NetworkQuality::Moderate,
            _ => NetworkQuality::Poor,
        }
    }

    /// Get recommended buffer size multiplier
    pub fn buffer_multiplier(&self) -> f64 {
        match self {
            NetworkQuality::Excellent => 0.8,
            NetworkQuality::Good => 1.0,
            NetworkQuality::Moderate => 1.5,
            NetworkQuality::Poor => 2.0,
        }
    }

    /// Get recommended warmup packet count
    pub fn warmup_packets(&self) -> usize {
        match self {
            NetworkQuality::Excellent => 2,
            NetworkQuality::Good => 3,
            NetworkQuality::Moderate => 5,
            NetworkQuality::Poor => 8,
        }
    }

    /// Get reorder tolerance window (in milliseconds)
    ///
    /// Each step is a whole number of 20 ms frames: 2, 4, 8 and 16 respectively.
    pub fn reorder_window_ms(&self) -> u64 {
        match self {
            NetworkQuality::Excellent => 40,
            NetworkQuality::Good => 80,
            NetworkQuality::Moderate => 160,
            NetworkQuality::Poor => 320,
        }
    }
}
