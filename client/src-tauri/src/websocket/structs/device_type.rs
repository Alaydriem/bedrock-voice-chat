use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Device type for mute commands
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    /// Input device (microphone)
    Input,
    /// Output device (speaker/headphones)
    Output,
}
