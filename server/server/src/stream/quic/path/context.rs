// Per-connection path count. Every event handler receives `&mut` access to its
// context, so no interior mutability is needed.
#[derive(Default)]
pub struct PathObserverContext {
    count: u32,
}

impl PathObserverContext {
    // s2n-quic permits five paths per connection and never reclaims an index
    // (`s2n-quic-transport::path::manager::MAX_ALLOWED_PATHS`). Warning one below
    // that is the last moment a log line arrives before datagrams from any further
    // source address are silently dropped.
    pub const NEAR_LIMIT_THRESHOLD: u32 = 4;

    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn record_path(&mut self) -> u32 {
        self.count = self.count.saturating_add(1);
        self.count
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn is_near_limit(count: u32) -> bool {
        count >= Self::NEAR_LIMIT_THRESHOLD
    }
}
