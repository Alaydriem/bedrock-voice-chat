use std::time::Duration;

/// How many concurrent voice sessions a server admits, and how long a departed identity
/// keeps its place.
///
/// Counted by canonical identity rather than by connection. The registry mints a
/// connection id per connection, so a reconnecting player holds two of them until the old
/// one closes, and counting connections would refuse them their own slot.
#[derive(Debug, Clone, Copy)]
pub struct CapacityPolicy {
    connections: u32,
    reconnect_grace: Duration,
}

impl CapacityPolicy {
    pub fn new(connections: u32, reconnect_grace: Duration) -> Self {
        Self {
            connections,
            reconnect_grace,
        }
    }

    pub fn limit(&self) -> u32 {
        self.connections
    }

    pub fn grace(&self) -> Duration {
        self.reconnect_grace
    }

    pub fn is_unlimited(&self) -> bool {
        self.connections == 0
    }
}
