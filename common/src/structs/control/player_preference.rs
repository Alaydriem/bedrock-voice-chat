use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PlayerPreference {
    pub owner: String,
    pub target: String,
    pub volume: f32,
    pub muted: bool,
}
