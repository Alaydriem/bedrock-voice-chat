use std::sync::Arc;

use uuid::Uuid;

use crate::config::Meridian;

// Present on every JSON line, null until config resolves them. A stable schema
// with a visible gap is easier to query than a key that is sometimes absent.
#[derive(Debug, Clone)]
pub struct ContextKeys {
    pub instance_id: Option<u16>,
    pub name: Option<String>,
    pub boot_id: String,
}

pub struct LogContext {
    keys: ContextKeys,
}

impl LogContext {
    pub fn new(meridian: Option<&Meridian>) -> Self {
        Self {
            keys: ContextKeys {
                instance_id: meridian.map(|m| m.instance_id),
                name: meridian.map(|m| m.name.clone()),
                // v7 rather than v4: the uuid dependency enables only v7, and a
                // time-ordered value suits a boot identifier anyway.
                boot_id: Uuid::now_v7().to_string(),
            },
        }
    }

    pub fn new_shared(meridian: Option<&Meridian>) -> Arc<Self> {
        Arc::new(Self::new(meridian))
    }

    pub fn snapshot(&self) -> ContextKeys {
        self.keys.clone()
    }
}
