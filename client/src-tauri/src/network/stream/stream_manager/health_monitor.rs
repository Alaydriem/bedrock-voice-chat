use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Health monitor state for tracking connection health
/// Used to detect disconnections and trigger reconnection
pub struct HealthMonitorState {
    /// Last time any packet was received (Unix timestamp in milliseconds)
    last_packet_received: AtomicU64,
    /// Whether we're waiting for a health check response
    awaiting_response: AtomicBool,
    /// Number of consecutive health check failures
    failure_count: AtomicU32,
}

impl HealthMonitorState {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            last_packet_received: AtomicU64::new(now),
            awaiting_response: AtomicBool::new(false),
            failure_count: AtomicU32::new(0),
        }
    }

    /// Called when any packet is received from the server
    /// Updates the last_packet_received timestamp
    pub fn on_packet_received(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_packet_received.store(now, Ordering::Relaxed);
    }

    /// Called when a health check response is received
    /// Clears the awaiting flag and resets failure count
    pub fn on_health_check_received(&self) {
        self.awaiting_response.store(false, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.on_packet_received();
    }

    /// Check if we should send a health check
    /// Returns true if no packets received for the threshold duration AND we're not already awaiting
    pub fn should_send_health_check(&self, threshold: Duration) -> bool {
        if self.awaiting_response.load(Ordering::Relaxed) {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = self.last_packet_received.load(Ordering::Relaxed);
        let elapsed_ms = now.saturating_sub(last);

        elapsed_ms >= threshold.as_millis() as u64
    }

    /// Set the awaiting response flag
    pub fn set_awaiting(&self, awaiting: bool) {
        self.awaiting_response.store(awaiting, Ordering::Relaxed);
    }

    /// Called after timeout waiting for health check response
    /// Increments failure count if still awaiting, returns the new count
    pub fn on_timeout(&self) -> u32 {
        if self.awaiting_response.load(Ordering::Relaxed) {
            let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
            self.awaiting_response.store(false, Ordering::Relaxed);
            count
        } else {
            self.failure_count.load(Ordering::Relaxed)
        }
    }

    /// Get the current failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Reset the health monitor state (e.g., on successful reconnect)
    pub fn reset(&self) {
        self.on_packet_received();
        self.awaiting_response.store(false, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
    }
}

impl Default for HealthMonitorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_new_state() {
        let state = HealthMonitorState::new();
        assert_eq!(state.failure_count(), 0);
        assert!(!state.should_send_health_check(Duration::from_secs(15)));
    }

    #[test]
    fn test_on_packet_received() {
        let state = HealthMonitorState::new();
        sleep(Duration::from_millis(10));
        state.on_packet_received();
        // Should not need health check immediately after receiving a packet
        assert!(!state.should_send_health_check(Duration::from_millis(100)));
    }

    #[test]
    fn test_should_send_health_check_threshold() {
        let state = HealthMonitorState::new();
        // Immediately after creation, should not need check
        assert!(!state.should_send_health_check(Duration::from_millis(50)));

        // After waiting past threshold, should need check
        sleep(Duration::from_millis(60));
        assert!(state.should_send_health_check(Duration::from_millis(50)));
    }

    #[test]
    fn test_awaiting_blocks_new_checks() {
        let state = HealthMonitorState::new();
        sleep(Duration::from_millis(60));

        // Should need check after threshold
        assert!(state.should_send_health_check(Duration::from_millis(50)));

        // Set awaiting
        state.set_awaiting(true);

        // Now should not send another check
        assert!(!state.should_send_health_check(Duration::from_millis(50)));
    }

    #[test]
    fn test_on_timeout_increments_failures() {
        let state = HealthMonitorState::new();
        state.set_awaiting(true);

        let count = state.on_timeout();
        assert_eq!(count, 1);
        assert_eq!(state.failure_count(), 1);

        state.set_awaiting(true);
        let count = state.on_timeout();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_on_health_check_received_resets() {
        let state = HealthMonitorState::new();
        state.set_awaiting(true);
        state.on_timeout(); // failure_count = 1
        state.set_awaiting(true);

        state.on_health_check_received();

        assert_eq!(state.failure_count(), 0);
        assert!(!state.should_send_health_check(Duration::from_millis(50)));
    }

    #[test]
    fn test_reset() {
        let state = HealthMonitorState::new();
        state.set_awaiting(true);
        state.on_timeout();
        state.on_timeout();

        state.reset();

        assert_eq!(state.failure_count(), 0);
        assert!(!state.should_send_health_check(Duration::from_millis(50)));
    }
}
