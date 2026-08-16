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
    #[serde(default = "default_true")]
    pub telemetry: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            openapi_docs: default_false(),
            telemetry: default_true(),
        }
    }
}
