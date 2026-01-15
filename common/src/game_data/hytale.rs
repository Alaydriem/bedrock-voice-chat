use serde::{Deserialize, Serialize};

/// Hytale-specific structs

/// Hytale world dimensions
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
pub enum Dimension {
    #[default]
    #[serde(rename = "orbis")]
    Orbis,
}
