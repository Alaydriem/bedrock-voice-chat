use serde::{Deserialize, Serialize};

/// The key for a per-player local preference: whose preference (`owner`) and about
/// whom (`target`). A concrete type rather than a bare `(String, String)` so call
/// sites read clearly and the key can grow behavior.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PreferenceKey {
    pub owner: String,
    pub target: String,
}

impl PreferenceKey {
    pub fn new(owner: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            target: target.into(),
        }
    }
}
