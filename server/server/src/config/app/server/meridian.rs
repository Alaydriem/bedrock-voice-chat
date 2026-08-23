use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Meridian {
    pub url: String,
    pub api_key: String,
    pub instance_id: u16,
    /// Stable registry record name, assigned per customer at provisioning time.
    ///
    /// Must not be generated at runtime. A fresh name per registration leaves an
    /// orphaned entry in Meridian's registry on every restart, because the record
    /// is keyed by name and nothing reclaims the old one.
    pub name: String,
    #[serde(default)]
    pub host: Option<String>,
    pub backend: String,
}
