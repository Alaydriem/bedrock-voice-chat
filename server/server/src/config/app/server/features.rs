use serde::{Deserialize, Serialize};

pub fn default_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    #[serde(default = "default_false")]
    pub openapi_docs: bool,
    #[serde(default)]
    pub relay: super::relay::RelayFeature,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            openapi_docs: default_false(),
            relay: super::relay::RelayFeature::default(),
        }
    }
}
