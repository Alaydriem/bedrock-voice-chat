
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel {
    // Minimal buffering needed
    None,
    // Slight buffer increase
    Light,
    // Significant buffer increase
    Moderate,
    // Maximum buffering plus an aggressive drop policy
    Severe,
}

impl CongestionLevel {
    /// Assess congestion from buffer metrics
    pub fn from_buffer_metrics(
        avg_depth: f64,
        target_depth: usize,
        underruns: u64,
        overflows: u64,
    ) -> Self {
        let depth_ratio = avg_depth / target_depth as f64;
        let total_issues = underruns + overflows;

        match (depth_ratio, total_issues) {
            (ratio, issues) if ratio < 0.5 && issues == 0 => CongestionLevel::None,
            (ratio, issues) if ratio < 1.5 && issues < 5 => CongestionLevel::Light,
            (ratio, issues) if ratio < 3.0 && issues < 20 => CongestionLevel::Moderate,
            _ => CongestionLevel::Severe,
        }
    }

    /// Get adjustment factor for buffer capacity
    pub fn capacity_adjustment(&self) -> f64 {
        match self {
            CongestionLevel::None => 0.9,
            CongestionLevel::Light => 1.0,
            CongestionLevel::Moderate => 1.3,
            CongestionLevel::Severe => 1.8,
        }
    }
}
