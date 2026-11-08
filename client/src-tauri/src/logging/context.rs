use std::sync::Arc;

use parking_lot::RwLock;

#[derive(Debug, Clone, Default)]
pub struct CorrelationKeys {}

pub struct LogContext {
    keys: RwLock<CorrelationKeys>,
}

impl LogContext {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(CorrelationKeys::default()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn snapshot(&self) -> CorrelationKeys {
        self.keys.read().clone()
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}
