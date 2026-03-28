pub mod server_permissions;
pub use server_permissions::ServerPermissions;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum Permission {
    #[serde(rename = "audio_upload")]
    AudioUpload,
    #[serde(rename = "audio_delete")]
    AudioDelete,
}

impl Permission {
    pub fn all() -> Vec<Permission> {
        vec![Permission::AudioUpload, Permission::AudioDelete]
    }

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
