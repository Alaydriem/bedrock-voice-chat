use serde::{Deserialize, Serialize};

/// Hytale-specific structs

/// Hytale world dimensions
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum Dimension {
    #[default]
    #[serde(rename = "orbis")]
    Orbis,
    #[serde(rename = "death")]
    Death,
}
