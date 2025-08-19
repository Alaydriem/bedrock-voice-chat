use super::{NetworkQuality, CongestionLevel, AdaptiveBufferState};
use crate::audio::stream::jitter_buffer::metrics::MetricsCollector;

/// Makes intelligent decisions about buffer management based on network conditions
#[derive(Debug)]
pub struct AdaptationEngine {
    pub state: AdaptiveBufferState,
    pub network_quality: NetworkQuality,
    pub congestion_level: CongestionLevel,
    base_capacity: usize,
}

impl AdaptationEngine {
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            state: AdaptiveBufferState::new(initial_capacity),
            network_quality: NetworkQuality::Good,
            congestion_level: CongestionLevel::None,
            base_capacity: initial_capacity,
        }
    }
    
    /// Assess current network conditions from metrics
    pub fn assess_network_conditions(&mut self, metrics: &MetricsCollector) -> NetworkQuality {
        let (loss_rate, jitter, avg_depth) = metrics.network_summary();
        
        // Update network quality assessment
        self.network_quality = NetworkQuality::from_metrics(
            loss_rate,
            jitter,
            metrics.network_metrics.rtt_variance
        );
        
        // Update congestion level
        self.congestion_level = CongestionLevel::from_buffer_metrics(
            avg_depth,
            metrics.network_metrics.target_buffer_depth,
            metrics.network_metrics.buffer_underruns,
            metrics.network_metrics.buffer_overflows
        );
        
        self.network_quality
    }
    
    /// Calculate target buffer capacity based on current conditions
    pub fn calculate_target_capacity(&self) -> usize {
        let network_multiplier = self.network_quality.buffer_multiplier();
        let congestion_multiplier = self.congestion_level.capacity_adjustment();
        
        // Combine both factors
        let combined_multiplier = network_multiplier * congestion_multiplier;
        
        self.state.calculate_target_capacity(self.base_capacity, combined_multiplier)
    }
    
    /// Determine if buffer should be adjusted
    pub fn should_adjust_buffer(&self, metrics: &MetricsCollector) -> bool {
        if !self.state.can_adjust() {
            return false;
        }
        
        let quality_score = metrics.quality_score();
        let target_capacity = self.calculate_target_capacity();
        
        // Adjust if quality is below threshold or capacity is significantly different
        let capacity_diff_ratio = (target_capacity as f64 - self.state.current_capacity as f64).abs() 
            / self.state.current_capacity as f64;
        
        quality_score < self.state.quality_threshold || capacity_diff_ratio > 0.15
    }
    
    /// Perform buffer adjustment if needed
    pub fn adjust_buffer_if_needed(&mut self, metrics: &MetricsCollector) -> Option<usize> {
        if !self.should_adjust_buffer(metrics) {
            return None;
        }
        
        let target_capacity = self.calculate_target_capacity();
        
        if self.state.adjust_capacity(target_capacity) {
            let recommended_warmup = self.network_quality.warmup_packets();
            self.state.update_warmup_requirements(recommended_warmup);
            
            Some(self.state.current_capacity)
        } else {
            None
        }
    }
    
    /// Get current capacity
    pub fn current_capacity(&self) -> usize {
        self.state.current_capacity
    }
    
    /// Get current warmup packet requirements
    pub fn warmup_packets_needed(&self) -> usize {
        self.state.warmup_packets_needed
    }
    
    /// Get reorder tolerance window based on network quality
    pub fn reorder_window_ms(&self) -> u64 {
        self.network_quality.reorder_window_ms()
    }
    
    /// Check if packet timestamp is acceptable given current network conditions
    pub fn is_timestamp_acceptable(&self, timestamp: u64, last_accepted: u64) -> bool {
        if timestamp <= last_accepted {
            return false;
        }
        
        let time_diff = timestamp.saturating_sub(last_accepted);
        let reorder_window = self.reorder_window_ms();
        
        if time_diff > 1000 {
            return true;
        }
        
        time_diff <= reorder_window
    }

}