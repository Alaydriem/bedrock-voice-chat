use std::time::Duration;

/// Configuration for health monitoring
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// How often to check if we need to send a health check
    pub check_interval: Duration,
    /// Send health check if no packets received for this duration
    pub threshold: Duration,
    /// How long to wait for health check response
    pub timeout: Duration,
    /// Number of consecutive failures before triggering reconnect
    pub max_failures: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(2),
            threshold: Duration::from_secs(5),
            timeout: Duration::from_secs(2),
            max_failures: 3,
        }
    }
}
