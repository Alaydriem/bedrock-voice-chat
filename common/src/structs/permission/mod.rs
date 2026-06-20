pub mod effect;
pub mod server_permissions;
pub use effect::PermissionEffect;
use serde::{Deserialize, Serialize};
pub use server_permissions::ServerPermissions;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum Permission {
    #[serde(rename = "audio_upload")]
    AudioUpload,
    #[serde(rename = "audio_delete")]
    AudioDelete,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "peer_link")]
    PeerLink,
}

impl Permission {
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::AudioUpload,
            Permission::AudioDelete,
            Permission::Admin,
            Permission::PeerLink,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::AudioUpload => "audio_upload",
            Permission::AudioDelete => "audio_delete",
            Permission::Admin => "admin",
            Permission::PeerLink => "peer_link",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        Self::all().into_iter().find(|p| p.as_str() == value)
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
