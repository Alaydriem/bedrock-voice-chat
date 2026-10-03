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
    #[serde(default = "default_true")]
    pub chat: bool,
    // Off stops the app pointing the idle ring at players beyond voice range. The feed still
    // carries them, so this shapes what the app shows rather than what a client can learn.
    #[serde(default = "default_true")]
    pub radar: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            openapi_docs: default_false(),
            telemetry: default_true(),
            chat: default_true(),
            radar: default_true(),
        }
    }
}
