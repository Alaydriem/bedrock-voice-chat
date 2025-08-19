#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkQuality {
    Excellent,  // < 1% loss, < 20ms jitter, stable RTT
    Good,       // < 3% loss, < 50ms jitter, moderate RTT variance  
    Moderate,   // < 8% loss, < 100ms jitter, high RTT variance
    Poor,       // > 8% loss, > 100ms jitter, very unstable
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
            NetworkQuality::Excellent => 0.8,  // Can use smaller buffer
            NetworkQuality::Good => 1.0,       // Normal buffer size
            NetworkQuality::Moderate => 1.5,   // Increase buffer
            NetworkQuality::Poor => 2.0,       // Large buffer for stability
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
    pub fn reorder_window_ms(&self) -> u64 {
        match self {
            NetworkQuality::Excellent => 40,   // 2 frames
            NetworkQuality::Good => 80,        // 4 frames  
            NetworkQuality::Moderate => 160,   // 8 frames
            NetworkQuality::Poor => 320,       // 16 frames
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel {
    None,       // Minimal buffering needed
    Light,      // Slight buffer increase
    Moderate,   // Significant buffer increase
    Severe,     // Maximum buffering + aggressive drop policy
}

impl CongestionLevel {
    /// Assess congestion from buffer metrics
    pub fn from_buffer_metrics(avg_depth: f64, target_depth: usize, underruns: u64, overflows: u64) -> Self {
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
