use serde::{Deserialize, Serialize};

/// Minecraft-specific structs

/// Minecraft world dimensions
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Dimension {
    #[serde(rename = "overworld")]
    Overworld,
    #[serde(rename = "the_end")]
    TheEnd,
    #[serde(rename = "nether")]
    TheNether,
}
