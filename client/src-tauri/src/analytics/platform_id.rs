use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

/// The anonymous install identity, shared by everything that reports under it:
/// the PostHog distinct id, the Sentry user, and the Flagsmith targeting key.
///
/// Held behind a lock rather than copied into each consumer because the About
/// pane can replace it. A consumer holding its own `String` would keep
/// reporting under the retired identity until the next launch.
pub struct PlatformId {
    value: RwLock<String>,
}

impl PlatformId {
    pub fn new(value: String) -> Self {
        Self {
            value: RwLock::new(value),
        }
    }

    pub fn new_shared(value: String) -> Arc<Self> {
        Arc::new(Self::new(value))
    }

    pub fn get(&self) -> String {
        self.value.read().clone()
    }

    pub fn set(&self, value: String) {
        *self.value.write() = value;
    }

    /// A version-4 id, deliberately. `InstallMarker` reads the install date out
    /// of the high bits of a version-7 id, so a time-ordered replacement would
    /// rewrite the recorded install date on the next launch and move the install
    /// into today's cohort.
    pub fn generate() -> String {
        Uuid::new_v4().to_string()
    }
}
