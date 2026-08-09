use serde::{Deserialize, Serialize};

pub fn default_false() -> bool {
    false
}

pub fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Features {
    #[serde(default = "default_false")]
    pub openapi_docs: bool,
    #[serde(default)]
    pub relay: super::relay::RelayFeature,
    #[serde(default = "default_true")]
    pub telemetry: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            openapi_docs: default_false(),
            relay: super::relay::RelayFeature::default(),
            telemetry: default_true(),
        }
    }
}
