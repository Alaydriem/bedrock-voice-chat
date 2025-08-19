use std::time::{Duration, Instant};
use super::{NetworkMetrics, DiagnosticMetrics};

/// Centralized metrics collection and reporting
#[derive(Debug)]
pub struct MetricsCollector {
    pub network_metrics: NetworkMetrics,
    pub diagnostic_metrics: DiagnosticMetrics,
    report_interval: Duration,
    last_report: Instant,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            network_metrics: NetworkMetrics::default(),
            diagnostic_metrics: DiagnosticMetrics::default(),
            report_interval: Duration::from_secs(30), // Report every 30 seconds
            last_report: Instant::now(),
        }
    }
}

impl MetricsCollector {
    pub fn new(report_interval: Duration) -> Self {
        Self {
            network_metrics: NetworkMetrics::default(),
            diagnostic_metrics: DiagnosticMetrics::default(),
            report_interval,
            last_report: Instant::now(),
        }
    }
    
    /// Record packet arrival and update network metrics
    pub fn record_packet_arrival(&mut self, timestamp: u64, buffer_depth: usize) {
        self.network_metrics.record_packet_arrival(timestamp, buffer_depth);
    }
    
    /// Record successful decode
    pub fn record_decode_success(&mut self, frames_decoded: usize) {
        self.diagnostic_metrics.record_decode(frames_decoded);
    }
    
    /// Record PLC generation
    pub fn record_plc_generation(&mut self) {
        self.diagnostic_metrics.record_plc();
    }
    
    /// Record silence generation
    pub fn record_silence_generation(&mut self) {
        self.diagnostic_metrics.record_silence();
    }
    
    /// Record buffer overflow drop
    pub fn record_overflow_drop(&mut self) {
        self.network_metrics.record_overflow();
        self.diagnostic_metrics.record_overflow_drop();
    }
    
    /// Record out-of-order drop
    pub fn record_ooo_drop(&mut self) {
        self.diagnostic_metrics.record_ooo_drop();
    }
    
    /// Record buffer underrun
    pub fn record_underrun(&mut self) {
        self.network_metrics.record_underrun();
    }
    
    /// Record ring buffer state update
    pub fn update_ring_metrics(&mut self, current_len: usize) {
        self.diagnostic_metrics.update_ring_metrics(current_len);
    }
    
    /// Record adaptation event
    pub fn record_adaptation(&mut self, timestamp: u64) {
        self.diagnostic_metrics.record_adaptation(timestamp);
    }
    
    /// Check if it's time to generate a report
    pub fn should_report(&self) -> bool {
        self.last_report.elapsed() >= self.report_interval
    }
    
    /// Generate comprehensive report
    pub fn generate_report(&mut self) -> String {
        self.last_report = Instant::now();
        
        let quality_score = self.diagnostic_metrics.calculate_quality_score();
        let loss_rate = self.network_metrics.packet_loss_rate();
        let jitter = self.network_metrics.jitter();
        
        format!(
            "JitterBuffer Report - Quality: {:.2} | Loss: {:.1}% | Jitter: {:.1}ms | {} | Network: rx={} underruns={} overflows={}",
            quality_score,
            loss_rate * 100.0,
            jitter,
            self.diagnostic_metrics.format_diagnostics(),
            self.network_metrics.packets_received,
            self.network_metrics.buffer_underruns,
            self.network_metrics.buffer_overflows
        )
    }
    
    /// Get current quality score
    pub fn quality_score(&self) -> f64 {
        self.diagnostic_metrics.calculate_quality_score()
    }
    
    /// Get network condition summary
    pub fn network_summary(&self) -> (f64, f64, f64) {
        (
            self.network_metrics.packet_loss_rate(),
            self.network_metrics.jitter(),
            self.network_metrics.avg_buffer_depth
        )
    }
}
