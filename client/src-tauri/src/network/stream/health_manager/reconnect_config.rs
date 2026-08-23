use std::time::Duration;

/// Configuration for reconnection probing
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first probe
    pub initial_delay: Duration,
    /// Maximum delay between probes
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Maximum number of probe attempts
    pub max_attempts: u32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(1_000),
            max_delay: Duration::from_millis(10_000),
            backoff_multiplier: 2.0,
            jitter_factor: 0.2,
            max_attempts: 20,
        }
    }
}
