use std::sync::Arc;

use parking_lot::RwLock;

#[derive(Debug, Clone, Default)]
pub struct CorrelationKeys {
    pub platform_id: Option<String>,
    pub install_id: Option<String>,
    pub session_id: Option<String>,
}

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

    pub fn set(&self, platform_id: String, install_id: String, session_id: String) {
        let mut keys = self.keys.write();
        keys.platform_id = Some(platform_id);
        keys.install_id = Some(install_id);
        keys.session_id = Some(session_id);
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
