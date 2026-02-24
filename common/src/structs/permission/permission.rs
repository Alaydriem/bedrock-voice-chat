use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Permission types for the ABAC permission system.
/// Used in DB storage (serde-renamed string), config defaults, CLI commands,
/// LoginResponse, and client-side permission checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum Permission {
    #[serde(rename = "audio_upload")]
    AudioUpload,
    #[serde(rename = "audio_delete")]
    AudioDelete,
}

impl Permission {
    /// Returns all permission variants for iteration.
    pub fn all() -> Vec<Permission> {
        vec![Permission::AudioUpload, Permission::AudioDelete]
    }

    /// Returns the serde-serialized string name for this permission.
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::AudioUpload => "audio_upload",
            Permission::AudioDelete => "audio_delete",
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
