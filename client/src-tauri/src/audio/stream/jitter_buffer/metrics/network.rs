use std::time::Instant;

/// Network condition tracking and analysis
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub rtt_variance: f64,
    pub packets_received: u64,
    pub packets_expected: u64,
    pub large_timestamp_jumps: u64,
    pub packet_loss_count: u64,
    pub buffer_underruns: u64,
    pub buffer_overflows: u64,
    pub avg_buffer_depth: f64,
    pub target_buffer_depth: usize,
    pub last_packet_timestamp: u64,
    pub last_update: Instant,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            rtt_variance: 0.0,
            packets_received: 0,
            packets_expected: 0,
            large_timestamp_jumps: 0,
            packet_loss_count: 0,
            buffer_underruns: 0,
            buffer_overflows: 0,
            avg_buffer_depth: 0.0,
            target_buffer_depth: 10,
            last_packet_timestamp: 0,
            last_update: Instant::now(),
        }
    }
}

impl NetworkMetrics {
    /// Record packet arrival with timestamp and current buffer depth
    pub fn record_packet_arrival(&mut self, timestamp: u64, buffer_depth: usize) {
        self.packets_received += 1;
        self.packets_expected += 1;

        // Update average buffer depth
        let depth = buffer_depth as f64;
        if self.packets_received == 1 {
            self.avg_buffer_depth = depth;
        } else {
            // Exponential moving average
            self.avg_buffer_depth = 0.9 * self.avg_buffer_depth + 0.1 * depth;
        }

        // Check for large timestamp jumps
        if self.last_packet_timestamp > 0 {
            let time_diff = timestamp.saturating_sub(self.last_packet_timestamp);
            if time_diff > 1000 {
                // > 1 second jump
                self.large_timestamp_jumps += 1;
            }
        }

        self.last_packet_timestamp = timestamp;
        self.last_update = Instant::now();
    }

    /// Record a buffer underrun event
    pub fn record_underrun(&mut self) {
        self.buffer_underruns += 1;
    }

    /// Record a buffer overflow event  
    pub fn record_overflow(&mut self) {
        self.buffer_overflows += 1;
    }

    /// Calculate packet loss rate
    pub fn packet_loss_rate(&self) -> f64 {
        if self.packets_expected == 0 {
            return 0.0;
        }

        let lost = self.packets_expected.saturating_sub(self.packets_received);
        lost as f64 / self.packets_expected as f64
    }

    /// Get current jitter (RTT variance)
    pub fn jitter(&self) -> f64 {
        self.rtt_variance.sqrt()
    }
}
