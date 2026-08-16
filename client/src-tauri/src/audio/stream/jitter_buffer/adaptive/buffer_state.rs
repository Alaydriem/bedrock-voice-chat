use common::structs::metrics::TransportKind;
use std::time::{Duration, Instant};

/// Dynamic buffer management and adaptation logic
#[derive(Debug, Clone)]
pub struct AdaptiveBufferState {
    pub current_capacity: usize,
    pub min_capacity: usize,
    pub max_capacity: usize,
    pub warmup_packets_needed: usize,
    pub adaptation_rate: f64,
    pub quality_threshold: f64,
    pub stability_window: Duration,
    pub last_adjustment: Instant,
    pub min_adjustment_interval: Duration,
    pub max_change_per_adjustment: f64,
}

impl Default for AdaptiveBufferState {
    fn default() -> Self {
        Self {
            // Capacities are in milliseconds: 6, 3 and 25 frames of 20 ms.
            current_capacity: 120,
            min_capacity: 60,
            max_capacity: 500,
            warmup_packets_needed: 3,

            adaptation_rate: 0.1,
            quality_threshold: 0.8,
            stability_window: Duration::from_secs(5),
            last_adjustment: Instant::now(),

            min_adjustment_interval: Duration::from_millis(500),
            max_change_per_adjustment: 0.2,
        }
    }
}

impl AdaptiveBufferState {
    /// The smallest buffer each transport can be asked to hold, in milliseconds.
    ///
    /// QUIC loses a datagram and moves on, so 60 ms of headroom is enough to absorb the
    /// jitter that remains. TCP instead retransmits, and the whole stream waits behind the
    /// lost segment: a fast retransmit costs about one round trip, which on a link worth
    /// using is well inside 140 ms. Below that floor the recovery arrives after the frame
    /// was due and a recoverable hiccup becomes an audible dropout.
    pub fn floor_ms(transport: TransportKind) -> usize {
        match transport {
            TransportKind::Quic => 60,
            TransportKind::WebSocket => 140,
        }
    }

    /// Where each transport starts before it has measured anything.
    fn initial_ms(transport: TransportKind) -> usize {
        match transport {
            TransportKind::Quic => 120,
            TransportKind::WebSocket => 180,
        }
    }

    pub fn new(initial_capacity: usize, transport: TransportKind) -> Self {
        let mut state = Self::default();
        state.min_capacity = Self::floor_ms(transport);
        state.current_capacity = initial_capacity
            .max(Self::initial_ms(transport))
            .clamp(state.min_capacity, state.max_capacity);
        state
    }

    /// Check if buffer adjustment is allowed (rate limiting)
    pub fn can_adjust(&self) -> bool {
        self.last_adjustment.elapsed() >= self.min_adjustment_interval
    }

    /// Calculate target capacity based on recommended multiplier
    pub fn calculate_target_capacity(&self, base_capacity: usize, multiplier: f64) -> usize {
        let target = (base_capacity as f64 * multiplier) as usize;
        target.clamp(self.min_capacity, self.max_capacity)
    }

    /// Apply gradual capacity adjustment
    pub fn adjust_capacity(&mut self, target_capacity: usize) -> bool {
        if !self.can_adjust() {
            return false;
        }

        let max_change = (self.current_capacity as f64 * self.max_change_per_adjustment) as usize;
        // Never adjust by less than a whole frame, or the buffer cannot move at all.
        let max_change = max_change.max(1);

        let new_capacity = if target_capacity > self.current_capacity {
            (self.current_capacity + max_change).min(target_capacity)
        } else {
            (self.current_capacity.saturating_sub(max_change)).max(target_capacity)
        };

        if new_capacity != self.current_capacity {
            self.current_capacity = new_capacity;
            self.last_adjustment = Instant::now();
            true
        } else {
            false
        }
    }

    /// Update warmup requirements based on network conditions
    pub fn update_warmup_requirements(&mut self, recommended_warmup: usize) {
        self.warmup_packets_needed = recommended_warmup.clamp(1, 10);
    }

    /// Check if system has been stable for the stability window
    pub fn is_stable(&self) -> bool {
        self.last_adjustment.elapsed() >= self.stability_window
    }
}
