use std::sync::Mutex;

use super::host_kind_sealed::HostKindSealed;
use crate::services::HostKind;

// Holds the host kind a mod reports before `bvc_server_start`.
//
// Kind and sealed flag share one mutex, so a set racing with start either lands
// before the seal or is refused. A host that never calls the setter leaves the
// slot at Unknown.
pub struct HostKindSlot {
    state: Mutex<(HostKind, bool)>,
}

impl HostKindSlot {
    pub fn new() -> Self {
        Self {
            state: Mutex::new((HostKind::Unknown, false)),
        }
    }

    pub fn set(&self, raw: &str) -> Result<HostKind, HostKindSealed> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.1 {
            return Err(HostKindSealed);
        }
        // An empty kind is a host that reported nothing, not the standalone binary.
        state.0 = if raw.trim().is_empty() {
            HostKind::Unknown
        } else {
            HostKind::parse(raw)
        };
        Ok(state.0)
    }

    pub fn seal(&self) -> HostKind {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.1 = true;
        state.0
    }
}

impl Default for HostKindSlot {
    fn default() -> Self {
        Self::new()
    }
}
