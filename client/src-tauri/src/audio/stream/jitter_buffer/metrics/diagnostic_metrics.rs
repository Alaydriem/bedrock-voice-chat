/// Performance and quality metrics for monitoring/debugging
#[derive(Debug, Clone, Default)]
pub struct DiagnosticMetrics {
    pub frames_decoded: u64,
    pub frames_plc: u64,
    pub frames_silence: u64,
    pub frames_dropped_overflow: u64,
    pub frames_dropped_ooo: u64,
    pub aggregated_decodes: u64,
    
    // New adaptive metrics
    pub adaptation_events: u64,
    pub quality_score: f64,
    pub buffer_adjustments: u64,
    pub last_adaptation_time: u64,
    
    // Buffer state tracking
    pub ring_high_water: usize,
    pub last_ring_len: usize,
}

impl DiagnosticMetrics {
    /// Record a successful decode operation
    pub fn record_decode(&mut self, frames_decoded: usize) {
        self.frames_decoded += frames_decoded as u64;
    }
    
    /// Record PLC generation
    pub fn record_plc(&mut self) {
        self.frames_plc += 1;
    }
    
    /// Record silence generation
    pub fn record_silence(&mut self) {
        self.frames_silence += 1;
    }
    
    /// Record packet dropped due to overflow
    pub fn record_overflow_drop(&mut self) {
        self.frames_dropped_overflow += 1;
    }
    
    /// Record packet dropped due to out-of-order
    pub fn record_ooo_drop(&mut self) {
        self.frames_dropped_ooo += 1;
    }
    
    /// Record aggregated decode (multiple frames in one packet)
    pub fn record_aggregated_decode(&mut self) {
        self.aggregated_decodes += 1;
    }
    
    /// Record an adaptation event
    pub fn record_adaptation(&mut self, timestamp: u64) {
        self.adaptation_events += 1;
        self.last_adaptation_time = timestamp;
    }
    
    /// Record a buffer size adjustment
    pub fn record_buffer_adjustment(&mut self) {
        self.buffer_adjustments += 1;
    }
    
    /// Update ring buffer watermarks
    pub fn update_ring_metrics(&mut self, current_len: usize) {
        self.last_ring_len = current_len;
        if current_len > self.ring_high_water {
            self.ring_high_water = current_len;
        }
    }
    
    /// Generate a formatted diagnostics string
    pub fn format_diagnostics(&self) -> String {
        format!(
            "decoded={} plc={} silence={} overflow={} ooo={} aggregated={} adaptations={} adjustments={}",
            self.frames_decoded,
            self.frames_plc,
            self.frames_silence,
            self.frames_dropped_overflow,
            self.frames_dropped_ooo,
            self.aggregated_decodes,
            self.adaptation_events,
            self.buffer_adjustments
        )
    }
    
    /// Calculate overall quality score (0.0 to 1.0)
    pub fn calculate_quality_score(&self) -> f64 {
        let total_frames = self.frames_decoded + self.frames_plc + self.frames_silence;
        if total_frames == 0 {
            return 1.0;
        }
        
        // Quality based on ratio of successfully decoded frames
        let success_rate = self.frames_decoded as f64 / total_frames as f64;
        
        // Penalty for excessive PLC
        let plc_penalty = (self.frames_plc as f64 / total_frames as f64) * 0.5;
        
        // Penalty for silence
        let silence_penalty = (self.frames_silence as f64 / total_frames as f64) * 0.8;
        
        (success_rate - plc_penalty - silence_penalty).max(0.0).min(1.0)
    }
}
