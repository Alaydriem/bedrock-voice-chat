use serde::{Deserialize, Serialize};

fn default_minimum() -> u8 {
    13
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Age {
    #[serde(default = "default_minimum")]
    pub minimum: u8,
}

impl Default for Age {
    fn default() -> Self {
        Self {
            minimum: default_minimum(),
        }
    }
}
